// Copyright © 2021-2024 HQS Quantum Simulations GmbH. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the
// License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either
// express or implied. See the License for the specific language governing permissions and
// limitations under the License.

use num_complex::Complex64;
use qoqo_calculator::CalculatorFloat;
use roqoqo::{operations::*, RoqoqoBackendError, RoqoqoError};
use typst::foundations::Value::Symbol;

use crate::RenderPragmas;

const EPSILON: f64 = 1e-6;

// Operations that are ignored by backend and do not throw an error.
const ALLOWED_OPERATIONS: &[&str; 3] = &["DefinitionFloat", "DefinitionComplex", "DefinitionUsize"];

/// Adds vectors to the circuit gates if needed to be able represent all the qubits.
///
/// # Arguments
///
/// * `circuit_gates` - A vector of all the gates vectors of the circuit.
/// * `qubits` - A vector of the qubits to represent.
fn add_qubits_vec(circuit_gates: &mut Vec<Vec<String>>, qubits: &[usize]) {
    while &circuit_gates.len() <= qubits.iter().max().unwrap_or(&0) {
        circuit_gates.push(Vec::new());
    }
}

/// Calculates the length on the image since some gate are not represented in typst
/// and therefore are not taking any space.
///
/// # Arguments
///
/// * `gates` - A vector of gates in typst representation.
///
/// # Returns
///
/// * `usize` - The total length the gates will take on the image.
pub(crate) fn effective_len(gates: &[String]) -> usize {
    gates.len()
        - gates
            .iter()
            .filter(|gate| {
                gate.contains("slice")
                    || gate.contains("gategroup")
                    || gate.contains("lstick")
                    || gate.contains("setwire")
            })
            .collect::<Vec<&String>>()
            .len()
}

/// Flattens the length of the gates vector for certain qubits in the circuit.
/// Used before adding a multiqubit gate on these qubits.
///
/// # Arguments
///
/// * `circuit_gates` - A vector of all the gates vectors of the circuit.
/// * `qubits` - A vector of the qubits to flatten.
fn flatten_qubits(circuit_gates: &mut [Vec<String>], qubits: &[usize]) {
    let max_len = qubits
        .iter()
        .map(|&qubit| effective_len(&circuit_gates[qubit]))
        .max()
        .unwrap_or(0);
    while qubits
        .iter()
        .map(|&qubit| effective_len(&circuit_gates[qubit]))
        .any(|length| length != max_len)
    {
        for &qubit in qubits.iter() {
            if effective_len(&circuit_gates[qubit]) < max_len {
                circuit_gates[qubit].push("1".to_owned());
            }
        }
    }
}

/// Pushes ones in the gates with index between min and max.
///
/// # Arguments
///
/// * `circuit_gates` - A vector of all the gates vectors of the circuit.
/// * `min` - The minimum index of the circuit
/// * `max` - The maximum index of the circuit
fn push_ones(circuit_gates: &mut [Vec<String>], min: usize, max: usize) {
    for gates in circuit_gates.iter_mut().take(max + 1).skip(min + 1) {
        gates.push("1".to_owned());
    }
}

/// Flattens the length of the gates vector for certain qubits in the circuit.
/// Used before adding a multiqubit gate on these qubits.
///
/// # Arguments
///
/// * `gate_vec_1` - A vector of gates vectors of the circuit.
/// * `gate_vec_2` - A second vector of gates vectors of the circuit.
/// * `vec_1_ind` - A vector of the indexes to flatten.
/// * `vec_2_ind` - A vector of the indexes to flatten.
pub(crate) fn flatten_multiple_vec(
    gate_vec_1: &mut [Vec<String>],
    gate_vec_2: &mut [Vec<String>],
    vec_1_ind: &[usize],
    vec_2_ind: &[usize],
) {
    let max_len = vec_1_ind
        .iter()
        .map(|&index| effective_len(&gate_vec_1[index]))
        .chain(
            vec_2_ind
                .iter()
                .map(|&boson| effective_len(&gate_vec_2[boson])),
        )
        .max()
        .unwrap_or(0);
    while vec_1_ind
        .iter()
        .map(|&index| effective_len(&gate_vec_1[index]))
        .chain(
            vec_2_ind
                .iter()
                .map(|&boson| effective_len(&gate_vec_2[boson])),
        )
        .any(|length| length != max_len)
    {
        for &index in vec_1_ind.iter() {
            if effective_len(&gate_vec_1[index]) < max_len {
                gate_vec_1[index].push("1".to_owned());
            }
        }
        for &boson in vec_2_ind.iter() {
            if effective_len(&gate_vec_2[boson]) < max_len {
                gate_vec_2[boson].push("1".to_owned());
            }
        }
    }
}

/// Formats a String to be accepted in a typst math expression.  
/// If the string doesn't represent a typst symbol then it will be put inside `"`.
///
/// # Arguments
///
/// * `str_value` - The string to be formatted.
///
/// # Returns
///
/// * `String` The formatted string.
fn format_symbol_str(str_value: &str) -> String {
    let (main_variant, sup) = str_value.split_once('.').unwrap_or((str_value, ""));
    let all_symbols = typst::symbols::sym();
    let symbol = all_symbols.scope().get(main_variant);
    match symbol {
        Some(Symbol(symbol))
            if sup.is_empty() || symbol.variants().any(|(variant, _repr)| variant.eq(sup)) =>
        {
            str_value.to_owned()
        }
        _ => format!("\"{str_value}\""),
    }
}

fn count_digits(num: f64) -> usize {
    let num_str: String = num
        .to_string()
        .chars()
        .skip_while(|&c| c == '.' || c == '0' || c == '-')
        .collect();
    let count = num_str.chars().filter(|&c| c != '.').count();
    count
}

/// Formats a calculatorFloat to be displayed in a typst representation.
///
/// # Arguments
///
/// * `calculator` - The CalculatorFloat to be formatted.
///
/// # Returns
///
/// * `String` The calculator's typst representation.
fn format_calculator(calculator: &CalculatorFloat) -> String {
    match calculator {
        CalculatorFloat::Float(float_value) => match float_value {
            v if (v - std::f64::consts::PI).abs() < EPSILON => "pi".to_owned(),
            v if (v + std::f64::consts::PI).abs() < EPSILON => "-pi".to_owned(),
            v if (v - std::f64::consts::FRAC_PI_2).abs() < EPSILON => "pi/2".to_owned(),
            v if (v + std::f64::consts::FRAC_PI_2).abs() < EPSILON => "-pi/2".to_owned(),
            v if (v - 3.0 * std::f64::consts::FRAC_PI_2).abs() < EPSILON => "(3pi)/2".to_owned(),
            v if (v + 3.0 * std::f64::consts::FRAC_PI_2).abs() < EPSILON => "-(3pi)/2".to_owned(),
            v if (v - std::f64::consts::FRAC_PI_3).abs() < EPSILON => "pi/3".to_owned(),
            v if (v + std::f64::consts::FRAC_PI_3).abs() < EPSILON => "-pi/3".to_owned(),
            v if (v - 2.0 * std::f64::consts::FRAC_PI_3).abs() < EPSILON => "(2pi)/3".to_owned(),
            v if (v + 2.0 * std::f64::consts::FRAC_PI_3).abs() < EPSILON => "-(2pi)/3".to_owned(),
            v if (v - std::f64::consts::FRAC_PI_4).abs() < EPSILON => "pi/4".to_owned(),
            v if (v + std::f64::consts::FRAC_PI_4).abs() < EPSILON => "-pi/4".to_owned(),
            v if (v - 3.0 * std::f64::consts::FRAC_PI_4).abs() < EPSILON => "(3pi)/4".to_owned(),
            v if (v + 3.0 * std::f64::consts::FRAC_PI_4).abs() < EPSILON => "-(3pi)/4".to_owned(),
            v if (v - std::f64::consts::SQRT_2).abs() < EPSILON => "sqrt(2)".to_owned(),
            v if (v + std::f64::consts::SQRT_2).abs() < EPSILON => "-sqrt(2)".to_owned(),
            v if (v - std::f64::consts::FRAC_1_SQRT_2).abs() < EPSILON => "1/sqrt(2)".to_owned(),
            v if (v + std::f64::consts::FRAC_1_SQRT_2).abs() < EPSILON => "-1/sqrt(2)".to_owned(),
            v if v.abs() <= 0.005 || v.abs() >= 1000. => match count_digits(*v) {
                0 => "0".to_owned(),
                1 => format!("\"{v:e}\""),
                2 => format!("\"{v:.1e}\""),
                _ => format!("\"{v:.2e}\""),
            },
            _ => {
                if float_value.fract() == 0.0 {
                    format!("{float_value:.0}")
                } else if (float_value * 10.0).fract() == 0.0 {
                    format!("{float_value:.1}")
                } else {
                    format!("{float_value:.2}")
                }
            }
        },
        CalculatorFloat::Str(str_value) => {
            let mut value = str_value.as_str();
            if str_value.ends_with(')') && str_value.starts_with('(') {
                let mut remove_bracket = 1;
                for c in str_value.chars().skip(1).take(str_value.len() - 2) {
                    match c {
                        '(' => remove_bracket += 1,
                        ')' => remove_bracket -= 1,
                        _ => (),
                    }
                    if remove_bracket == 0 {
                        break;
                    }
                }
                if remove_bracket != 0 {
                    value = value.strip_prefix('(').unwrap_or(value);
                    value = value.strip_suffix(')').unwrap_or(value);
                }
            }
            let re = regex::Regex::new(r"([a-zA-Z][\w.]+|-?[\d\.]+e-?\d)").unwrap();
            re.replace_all(value, |caps: &regex::Captures| format_symbol_str(&caps[0]))
                .into()
        }
    }
}

/// Formats a complex value to be displayed in a typst representation.
///
/// # Arguments
///
/// * `value` - The complex value to be formatted
///
/// # Returns
///
/// * `String` - The complex's typst representation.
fn format_complex_value(value: Complex64) -> String {
    format!(
        "{}+{}i",
        format_calculator(&CalculatorFloat::Float(value.re)),
        format_calculator(&CalculatorFloat::Float(value.im))
    )
}

/// Formats a qubit value to be displayed as an input in a quill multi qubit gate.
///
/// # Arguments
///
/// * `qubit` - The qubit acting on the gate.
/// * `label` - The label to be displayed.
///
/// # Returns
///
/// * `String` - The formatted input string.
fn format_qubit_input(qubit: usize, label: &str) -> String {
    format!(r#"{qubit}, label: "{label}""#)
}

/// Prepares the circuit for a slice gate.
///
/// # Arguments
///
/// * `circuit_gates` - A vector of all the gates vectors of the circuit.
fn prepare_for_slice(circuit_gates: &mut Vec<Vec<String>>, circuit_lock: &mut Vec<(usize, usize)>) {
    add_qubits_vec(circuit_gates, &[0]);
    if circuit_gates[0].is_empty() {
        circuit_gates[0].push("1".to_owned());
        for qubit in 1..10 {
            circuit_lock.push((qubit, 0))
        }
    } else {
        let last_slice = circuit_gates[0]
            .iter()
            .filter(|gate| gate.contains("slice") || gate.contains("gategroup"))
            .next_back();
        if let Some(last_slice) = last_slice {
            let dist_to_max = circuit_gates
                .iter()
                .map(|gates: &Vec<String>| effective_len(gates) + 1)
                .max()
                .unwrap_or(0)
                - effective_len(&circuit_gates[0])
                + if last_slice.contains("gategroup") {
                    last_slice.split(",").nth(1).unwrap_or_default()
                } else {
                    Default::default()
                }
                .trim()
                .parse::<usize>()
                .unwrap_or_default();
            let len_to_add = match circuit_gates[0]
                .iter()
                .rev()
                .position(|gate| gate.eq(last_slice))
                .unwrap()
                + dist_to_max
            {
                0 => 5,
                1 => 3,
                2 => 1,
                _ => 0,
            };

            for _ in 0..(len_to_add + dist_to_max) {
                circuit_gates[0].push("1".to_owned());
            }
        }
    }
}

/// Prepares the circuit for a control gate.
///
/// # Arguments
///
/// * `circuit_gates` - A vector of all the gates vectors of the circuit.
/// * `circuit_lock` - The list of all the emplacements of the circuit that are reserved for a control wire between two gates.
fn prepare_for_ctrl(
    circuit_gates: &mut Vec<Vec<String>>,
    circuit_lock: &mut Vec<(usize, usize)>,
    qubits: &[usize],
) {
    let min = qubits.iter().min().unwrap().to_owned();
    let max = qubits.iter().max().unwrap().to_owned();
    add_qubits_vec(circuit_gates, qubits);
    flatten_qubits(circuit_gates, qubits);
    for qubit in min + 1..max {
        while circuit_lock.contains(&(qubit, effective_len(&circuit_gates[qubit]))) {
            circuit_lock.retain(|&val| val != (qubit, effective_len(&circuit_gates[qubit])));
            circuit_gates[qubit].push("1".to_owned());
        }

        if circuit_gates.len() > qubit
            && effective_len(&circuit_gates[qubit]) > effective_len(&circuit_gates[min])
        {
            flatten_qubits(circuit_gates, &[min, qubit]);
        }
    }
    flatten_qubits(circuit_gates, qubits);
    for qubit in min + 1..max {
        circuit_lock.push((qubit, effective_len(&circuit_gates[min])));
    }
}

fn prepare_for_bosonic(
    mode: usize,
    bosonic_gates: &mut [Vec<String>],
    bosonic_lock: &mut Vec<(usize, usize)>,
) {
    while bosonic_lock.contains(&(mode, effective_len(&bosonic_gates[mode]))) {
        bosonic_lock.retain(|&val| val != (mode, effective_len(&bosonic_gates[mode])));
        bosonic_gates[mode].push("1".to_owned());
    }
}

#[inline]
fn qubit_range_vec(qubits: &[usize]) -> Vec<usize> {
    (*qubits.iter().min().unwrap()..=*qubits.iter().max().unwrap()).collect()
}

#[inline]
fn qubit_range(qubits: &[usize]) -> usize {
    qubits.iter().max().unwrap() - qubits.iter().min().unwrap() + 1
}

/// Adds a gate to the circuit's typst representation.
///
/// # Arguments
///
/// * `circuit_gates` - A vector of all the gates vectors of the circuit.
/// * `bosonic_gates` - A vector of all the bosonic gates vectors of the circuit.
/// * `classical_gates` - A vector of all the operations on classical registers of the circuit.
/// * `circuit_lock` - The list of all the emplacements of the circuit that are reserved for a control wire between two gates.
/// * `bosonic_lock` - The list of all the emplacements of the bosonic part of the circuit that are reserved for a control wire between two gates.
/// * `classical_lock` - The list of all the emplacements of the classical part of the circuit that are reserved for a control wire between two gates.
/// * `operation` - The operation to add to the circuit.
///
/// # Returns
///
/// * `Ok(())` - If the operation was successfully added to the circuit.
/// * Err(RoqoqoBackendError) - Operation not supported.
#[allow(clippy::too_many_arguments)]
pub fn add_gate(
    circuit_gates: &mut Vec<Vec<String>>,
    bosonic_gates: &mut Vec<Vec<String>>,
    classical_gates: &mut Vec<Vec<String>>,
    circuit_lock: &mut Vec<(usize, usize)>,
    bosonic_lock: &mut Vec<(usize, usize)>,
    classical_lock: &mut Vec<(usize, usize)>,
    operation: &Operation,
    render_pragmas: &RenderPragmas,
) -> Result<(), RoqoqoBackendError> {
    match render_pragmas {
        RenderPragmas::All => (),
        RenderPragmas::None => {
            if operation.hqslang().starts_with("Pragma") {
                return Ok(());
            }
        }
        RenderPragmas::Partial(ref pragmas) => {
            if operation.hqslang().starts_with("Pragma")
                && !pragmas.contains(&operation.hqslang().to_owned())
            {
                return Ok(());
            }
        }
    }
    let mut used_qubits: Vec<usize> = Vec::new();
    match operation.involved_qubits() {
        InvolvedQubits::Set(involved_qubits) => {
            for qubit in involved_qubits.iter() {
                if !used_qubits.contains(qubit) {
                    used_qubits.push(*qubit);
                }
            }
        }
        InvolvedQubits::All => {
            for qubit in 0..circuit_gates.len() {
                if !used_qubits.contains(&qubit) {
                    used_qubits.push(qubit);
                }
            }
        }
        InvolvedQubits::None => {}
    }
    add_qubits_vec(circuit_gates, &used_qubits);
    flatten_qubits(circuit_gates, &used_qubits);
    for qubit in used_qubits.iter() {
        while circuit_lock.contains(&(*qubit, effective_len(&circuit_gates[*qubit]))) {
            circuit_lock.retain(|&val| val != (*qubit, effective_len(&circuit_gates[*qubit])));
            circuit_gates[*qubit].push("1".to_owned());
        }
    }
    match operation {
        Operation::Hadamard(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ H $".to_owned());
            Ok(())
        }
        Operation::CNOT(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push("targ()".to_owned());

            Ok(())
        }
        Operation::SingleQubitGate(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ U({}+{}i,{}+{}i,{}) $, label: \"SingleQubitGate\")",
                format_calculator(&op.alpha_r()),
                format_calculator(&op.alpha_i()),
                format_calculator(&op.beta_r()),
                format_calculator(&op.beta_i()),
                format_calculator(&op.global_phase())
            ));
            Ok(())
        }
        Operation::RotateX(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"Rx\"({}) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::RotateY(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"Ry\"({}) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::RotateZ(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"Rz\"({}) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::PauliX(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ X $".to_owned());
            Ok(())
        }
        Operation::PauliY(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ Y $".to_owned());
            Ok(())
        }
        Operation::PauliZ(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ Z $".to_owned());
            Ok(())
        }
        Operation::SqrtPauliX(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ sqrt(X) $".to_owned());
            Ok(())
        }
        Operation::InvSqrtPauliX(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ sqrt(X)^(dagger) $".to_owned());
            Ok(())
        }
        Operation::SGate(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ S $".to_owned());
            Ok(())
        }
        Operation::TGate(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ T $".to_owned());
            Ok(())
        }
        Operation::PhaseShiftState1(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"p1\"({}) $, label: \"PhaseShiftState1\")",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::PhaseShiftState0(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"p0\"({}) $, label: \"PhaseShiftState0\")",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::RotateAroundSphericalAxis(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"Rsph\"({},{},{}) $, label: \"RotateAroundSphericalAxis\")",
                format_calculator(op.theta()),
                format_calculator(op.spherical_theta()),
                format_calculator(op.spherical_phi()),
            ));
            Ok(())
        }
        Operation::RotateXY(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"Rxy\"({},{}) $)",
                format_calculator(op.theta()),
                format_calculator(op.phi())
            ));
            Ok(())
        }
        Operation::PragmaSetNumberOfMeasurements(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let n_qubits = circuit_gates.len();
            flatten_qubits(circuit_gates, &(0..n_qubits).collect::<Vec<usize>>());
            circuit_gates[0].push(format!(
                "slice(label: $ \"Measurements\"\\ \"n={}\" $)",
                op.number_measurements(),
            ));
            Ok(())
        }
        Operation::PragmaSetStateVector(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let n_qubits = circuit_gates.len();
            flatten_qubits(circuit_gates, &(0..n_qubits).collect::<Vec<usize>>());
            circuit_gates[0].push(format!(
                r#"slice(label: $ "SetStatevector"\ [{}] $, stroke: (paint: black, thickness: 1pt, dash: "solid"))"#,
                op.statevector().iter().map(|&complex| format_complex_value(complex)).collect::<Vec<String>>().join(","),
            ));
            Ok(())
        }
        Operation::PragmaSetDensityMatrix(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let n_qubits = circuit_gates.len();
            flatten_qubits(circuit_gates, &(0..n_qubits).collect::<Vec<usize>>());
            circuit_gates[0].push(format!(
                r#"slice(label: $ "SetDensityMatrix"\ "{}" $, stroke: (paint: black, thickness: 1pt, dash: "solid"))"#,
                op.density_matrix(),
            ));
            Ok(())
        }
        Operation::PragmaRepeatGate(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let n_qubits = circuit_gates.len();
            flatten_qubits(circuit_gates, &(0..n_qubits).collect::<Vec<usize>>());
            circuit_gates[0].push(format!(
                r#"slice(label: $ "RepeatNextGate"\ {}" times" $, stroke: (paint: black, thickness: 1pt, dash: "densely-dash-dotted"))"#,
                op.repetition_coefficient(),
            ));
            Ok(())
        }
        Operation::PragmaOverrotation(op) => {
            if op.qubits().is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = op.qubits().iter().min().unwrap().to_owned();
            let max = op.qubits().iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubits);
            circuit_gates[min].push(format!(
                r#"mqgate($ "Overrotation"\ ({},{})\ "\"{}\"" $, n: {}, width: 10em, fill: gray, inputs: ({}))"#,
                format_calculator(&CalculatorFloat::Float(*op.amplitude())),
                format_calculator(&CalculatorFloat::Float(*op.variance())),
                op.gate_hqslang(),
                qubits.len(),
                op.qubits().iter().map(|qubit| format!("(qubit: {})", format_qubit_input(qubit - min, "x"))).collect::<Vec<String>>().join(",")
            ));
            push_ones(circuit_gates, min, max);
            Ok(())
        }
        Operation::PragmaBoostNoise(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let n_qubits = circuit_gates.len();
            flatten_qubits(circuit_gates, &(0..n_qubits).collect::<Vec<usize>>());
            circuit_gates[0].push(format!(
                r#"slice(label: $ "BoostNoise"\ n={} $)"#,
                format_calculator(op.noise_coefficient()),
            ));
            Ok(())
        }
        Operation::PragmaStopParallelBlock(op) => {
            if op.qubits().is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = op.qubits().iter().min().unwrap().to_owned();
            let max = op.qubits().iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubits);
            circuit_gates[min].push(format!(
                r#"mqgate($ "StopParallelBlock"\ ({}) $, n: {}, width: 13em, fill: gray, inputs: ({}))"#,
                format_calculator(op.execution_time()),
                qubits.len(),
                op.qubits().iter().map(|qubit| format!("(qubit: {})", format_qubit_input(qubit - min, "x"))).collect::<Vec<String>>().join(",")
            ));
            push_ones(circuit_gates, min, max);
            Ok(())
        }
        Operation::PragmaStartDecompositionBlock(op) => {
            if op.qubits().is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = op.qubits().iter().min().unwrap().to_owned();
            let max = op.qubits().iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubits);
            circuit_gates[min].push(format!(
                r#"mqgate($ "StartDecompositionBlock"\ "{}" $, n: {}, width: 14em, fill: gray, inputs: ({}))"#,
                op.reordering_dictionary().iter().map(|(key, val)| format!("{key}:{val}")).collect::<Vec<String>>().join("\n"),
                qubits.len(),
                op.qubits().iter().map(|qubit| format!("(qubit: {})", format_qubit_input(qubit - min, "x"))).collect::<Vec<String>>().join(",")
            ));
            push_ones(circuit_gates, min, max);
            Ok(())
        }
        Operation::PragmaStopDecompositionBlock(op) => {
            if op.qubits().is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = op.qubits().iter().min().unwrap().to_owned();
            let max = op.qubits().iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubits);
            circuit_gates[min].push(format!(
                r#"mqgate($ "StopDecompositionBlock" $, n: {}, width: 13em, fill: gray, inputs: ({}))"#,
                qubits.len(),
                op.qubits().iter().map(|qubit| format!("(qubit: {})", format_qubit_input(qubit - min, "x"))).collect::<Vec<String>>().join(",")
            ));
            push_ones(circuit_gates, min, max);
            Ok(())
        }
        Operation::PragmaGlobalPhase(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let n_qubits = circuit_gates.len();
            flatten_qubits(circuit_gates, &(0..n_qubits).collect::<Vec<usize>>());
            circuit_gates[0].push(format!(
                r#"slice(label: $ "GlobalPhase"\ {} $)"#,
                format_calculator(op.phase()),
            ));
            Ok(())
        }
        Operation::PragmaSleep(op) => {
            if op.qubits().is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = op.qubits().iter().min().unwrap().to_owned();
            let max = op.qubits().iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubits);
            circuit_gates[min].push(format!(
                r#"mqgate($ "Sleep"({}) $, n: {}, width: 7em, fill: gray, inputs: ({}))"#,
                format_calculator(op.sleep_time()),
                qubits.len(),
                op.qubits()
                    .iter()
                    .map(|qubit| format!("(qubit: {})", format_qubit_input(qubit - min, "x")))
                    .collect::<Vec<String>>()
                    .join(",")
            ));
            push_ones(circuit_gates, min, max);
            Ok(())
        }
        Operation::PragmaActiveReset(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("gate($ \"Reset\" $, fill: gray)".to_owned());
            Ok(())
        }
        Operation::PragmaDamping(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"Damping\"({},{}) $, fill: gray)",
                format_calculator(op.gate_time()),
                format_calculator(op.rate()),
            ));
            Ok(())
        }
        Operation::PragmaDepolarising(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"Depolarising\"({},{}) $, fill: gray)",
                format_calculator(op.gate_time()),
                format_calculator(op.rate()),
            ));
            Ok(())
        }
        Operation::PragmaDephasing(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"Dephasing\"({},{}) $, fill: gray)",
                format_calculator(op.gate_time()),
                format_calculator(op.rate()),
            ));
            Ok(())
        }
        Operation::PragmaRandomNoise(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"RandomNoise\"({},{},{}) $, fill: gray)",
                format_calculator(op.gate_time()),
                format_calculator(op.depolarising_rate()),
                format_calculator(op.dephasing_rate()),
            ));
            Ok(())
        }
        Operation::PragmaGeneralNoise(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"GeneralNoise\"({},{}) $, fill: gray)",
                format_calculator(op.gate_time()),
                op.rates(),
            ));
            Ok(())
        }
        Operation::PragmaConditional(op) => {
            if op.circuit().is_empty() {
                return Ok(());
            }
            prepare_for_slice(circuit_gates, circuit_lock);
            let mut used_qubits: Vec<usize> = Vec::new();
            match op.involved_qubits() {
                InvolvedQubits::Set(involved_qubits) => {
                    for qubit in involved_qubits.iter() {
                        if !used_qubits.contains(qubit) {
                            used_qubits.push(*qubit);
                        }
                    }
                }
                InvolvedQubits::All => {
                    for qubit in 0..circuit_gates.len() {
                        if !used_qubits.contains(&qubit) {
                            used_qubits.push(qubit);
                        }
                    }
                }
                InvolvedQubits::None => {}
            }
            let min = used_qubits.iter().min().unwrap_or(&0_usize).to_owned();
            let max = used_qubits.iter().max().unwrap_or(&0_usize).to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            if qubits.is_empty() {
                return Ok(());
            }
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            circuit_gates[min].push(format!(
                "gategroup({}, replace_by_len, label: \"Conditional: {}[{}]\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                op.condition_register(),
                op.condition_index(),
            ));
            let group_index = circuit_gates[min].len() - 1;
            let old_len = circuit_gates
                .iter()
                .map(|gates| effective_len(gates))
                .collect::<Vec<usize>>();
            for operation in op.circuit().iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    operation,
                    render_pragmas,
                )?;
            }
            let max_gates_len_diff = qubits
                .iter()
                .map(|&qubit| effective_len(&circuit_gates[qubit]) - old_len[qubit])
                .max()
                .unwrap_or(0);
            circuit_gates[min][group_index] = circuit_gates[min][group_index]
                .replace("replace_by_len", &max_gates_len_diff.to_string());
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::PragmaChangeDevice(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let n_qubits = circuit_gates.len();
            flatten_qubits(circuit_gates, &(0..n_qubits).collect::<Vec<usize>>());
            circuit_gates[0].push(format!(
                r#"slice(label: $ "ChangeDevice"\ \"{}\" $)"#,
                op.wrapped_hqslang,
            ));
            Ok(())
        }
        Operation::SWAP(op) => {
            let min = *op.control().min(op.target());
            let max = *op.control().max(op.target());
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[min].push(format!("swap({})", max - min));
            circuit_gates[max].push("targX()".to_owned());
            Ok(())
        }
        Operation::ISwap(op) => {
            let min = *op.control().min(op.target());
            let max = *op.control().max(op.target());
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[min].push(format!("swap({}, label: \"ISwap\")", max - min));
            circuit_gates[max].push("targX()".to_owned());
            Ok(())
        }
        Operation::FSwap(op) => {
            let min = *op.control().min(op.target());
            let max = *op.control().max(op.target());
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[min].push(format!("swap({}, label: \"FSwap\")", max - min));
            circuit_gates[max].push("targX()".to_owned());
            Ok(())
        }
        Operation::SqrtISwap(op) => {
            let min = *op.control().min(op.target());
            let max = *op.control().max(op.target());
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[min].push(format!("swap({}, label: $ sqrt(\"ISwap\") $)", max - min));
            circuit_gates[max].push("targX()".to_owned());
            Ok(())
        }
        Operation::InvSqrtISwap(op) => {
            let min = *op.control().min(op.target());
            let max = *op.control().max(op.target());
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[min].push(format!(
                "swap({}, label: $ sqrt(\"ISwap\")^(dagger))",
                max - min
            ));
            circuit_gates[max].push("targX()".to_owned());
            Ok(())
        }
        Operation::XY(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"XY\"({})$)",
                format_calculator(op.theta()),
            ));
            Ok(())
        }
        Operation::ControlledPhaseShift(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"PhaseShift\"({}) $)",
                format_calculator(op.theta()),
            ));
            Ok(())
        }
        Operation::ControlledPauliY(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push("gate($ \"Y\" $)".to_string());
            Ok(())
        }
        Operation::ControlledPauliZ(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push("gate($ \"Z\" $)".to_string());
            Ok(())
        }
        Operation::MolmerSorensenXX(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push("gate($ \"MolmerSorensenXX\" $)".to_owned());
            Ok(())
        }
        Operation::VariableMSXX(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"VariableMSXX\"({}) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::GivensRotation(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"GivensRotation\"({},{}) $)",
                format_calculator(op.theta()),
                format_calculator(op.phi())
            ));
            Ok(())
        }
        Operation::GivensRotationLittleEndian(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"GivensRotationLE\"({},{}) $)",
                format_calculator(op.theta()),
                format_calculator(op.phi())
            ));
            Ok(())
        }
        Operation::Qsim(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"Qsim\"({},{},{}) $)",
                format_calculator(op.x()),
                format_calculator(op.y()),
                format_calculator(op.z())
            ));
            Ok(())
        }
        Operation::Fsim(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"Fsim\"({},{},{}) $)",
                format_calculator(op.t()),
                format_calculator(op.u()),
                format_calculator(op.delta()),
            ));
            Ok(())
        }
        Operation::SpinInteraction(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"SpinInteraction\"({},{},{}) $)",
                format_calculator(op.x()),
                format_calculator(op.y()),
                format_calculator(op.z())
            ));
            Ok(())
        }
        Operation::Bogoliubov(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"Bogoliubov\"({},{}) $)",
                format_calculator(op.delta_real()),
                format_calculator(op.delta_imag()),
            ));
            Ok(())
        }
        Operation::PMInteraction(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"PMInteraction\"({}) $)",
                format_calculator(op.t()),
            ));
            Ok(())
        }
        Operation::ComplexPMInteraction(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"ComplexPMInteraction\"({},{}) $)",
                format_calculator(op.t_real()),
                format_calculator(op.t_imag()),
            ));
            Ok(())
        }
        Operation::PhaseShiftedControlledZ(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"PhaseShiftedControlledZ\"({}) $)",
                format_calculator(op.phi()),
            ));
            Ok(())
        }
        Operation::MultiQubitMS(op) => {
            if op.qubits().is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = op.qubits().iter().min().unwrap().to_owned();
            let max = op.qubits().iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubits);
            circuit_gates[min].push(format!(
                r#"mqgate($ "MultiQubitMS"({}) $, n: {}, width: 11em, inputs: ({}))"#,
                format_calculator(op.theta()),
                qubits.len(),
                op.qubits()
                    .iter()
                    .map(|qubit| format!("(qubit: {})", format_qubit_input(qubit - min, "x")))
                    .collect::<Vec<String>>()
                    .join(",")
            ));
            for gates in circuit_gates.iter_mut().take(max + 1).skip(min + 1) {
                gates.push("1".to_owned());
            }
            Ok(())
        }
        Operation::MultiQubitZZ(op) => {
            if op.qubits().is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = op.qubits().iter().min().unwrap().to_owned();
            let max = op.qubits().iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubits);
            circuit_gates[min].push(format!(
                r#"mqgate($ "MultiQubitZZ"({}) $, n: {}, width: 11em, inputs: ({}))"#,
                format_calculator(op.theta()),
                qubits.len(),
                op.qubits()
                    .iter()
                    .map(|qubit| format!("(qubit: {})", format_qubit_input(qubit - min, "x")))
                    .collect::<Vec<String>>()
                    .join(",")
            ));
            for gates in circuit_gates.iter_mut().take(max + 1).skip(min + 1) {
                gates.push("1".to_owned());
            }
            Ok(())
        }
        Operation::MeasureQubit(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            if let Some((index, _)) = classical_gates
                .iter()
                .cloned()
                .enumerate()
                .find(|(_i, gates)| gates[0].eq(&format!("lstick($ \"{} : \" $)", op.readout())))
            {
                flatten_multiple_vec(circuit_gates, classical_gates, &[*op.qubit()], &[index]);
                for qubit in *op.qubit()..circuit_gates.len() {
                    while circuit_lock.contains(&(qubit, effective_len(&circuit_gates[qubit]))) {
                        circuit_lock
                            .retain(|&val| val != (qubit, effective_len(&circuit_gates[qubit])));
                        circuit_gates[qubit].push("1".to_owned());
                    }
                    if circuit_gates.len() > qubit
                        && effective_len(&circuit_gates[qubit])
                            > effective_len(&circuit_gates[*op.qubit()])
                    {
                        flatten_qubits(circuit_gates, &[*op.qubit(), qubit]);
                    }
                }
                for boson in 0..bosonic_gates.len() {
                    prepare_for_bosonic(boson, bosonic_gates, bosonic_lock);
                    if bosonic_gates.len() > boson
                        && effective_len(&bosonic_gates[boson])
                            > effective_len(&circuit_gates[*op.qubit()])
                    {
                        flatten_multiple_vec(
                            circuit_gates,
                            bosonic_gates,
                            &[*op.qubit()],
                            &[boson],
                        );
                    }
                }
                for classical_index in 0..index + 1 {
                    while classical_lock.contains(&(
                        classical_index,
                        effective_len(&classical_gates[classical_index]),
                    )) {
                        classical_lock.retain(|&val| {
                            val != (
                                classical_index,
                                effective_len(&classical_gates[classical_index]),
                            )
                        });
                        classical_gates[classical_index].push("1".to_owned());
                    }
                    if classical_gates.len() > classical_index
                        && effective_len(&classical_gates[classical_index])
                            > effective_len(&classical_gates[index])
                    {
                        flatten_qubits(classical_gates, &[index, classical_index]);
                    }
                }
                flatten_multiple_vec(circuit_gates, classical_gates, &[*op.qubit()], &[index]);
                for qubit in *op.qubit()..circuit_gates.len() + 10 {
                    circuit_lock.push((qubit, effective_len(&classical_gates[index])));
                }
                for boson in 0..bosonic_gates.len() + 10 {
                    bosonic_lock.push((boson, effective_len(&classical_gates[index])));
                }
                for classical_index in 0..index {
                    classical_lock.push((classical_index, classical_gates[index].len()));
                }
                circuit_gates[*op.qubit()].push(format!(
                    "meter(target:replace_by_classical_len_{}-{})",
                    index,
                    *op.qubit()
                ));
                classical_gates[index].push(format!(
                    "ctrl(0, label: (content: $ {} $, pos: bottom))",
                    op.readout_index()
                ))
            } else {
                circuit_gates[*op.qubit()].push("meter()".to_owned());
            }
            Ok(())
        }
        Operation::PragmaGetStateVector(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let circuit = op.circuit().clone().unwrap_or(
                (0..circuit_gates.len())
                    .map(|qubit| Operation::from(Identity::new(qubit)))
                    .collect(),
            );
            let mut used_qubits: Vec<usize> = Vec::new();
            match circuit.involved_qubits() {
                InvolvedQubits::Set(involved_qubits) => {
                    for qubit in involved_qubits.iter() {
                        if !used_qubits.contains(qubit) {
                            used_qubits.push(*qubit);
                        }
                    }
                }
                InvolvedQubits::All => {
                    for qubit in 0..circuit_gates.len() {
                        if !used_qubits.contains(&qubit) {
                            used_qubits.push(qubit);
                        }
                    }
                }
                InvolvedQubits::None => {}
            }
            if used_qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = used_qubits.iter().min().unwrap().to_owned();
            let max = used_qubits.iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            if circuit.is_empty() || qubits.is_empty() {
                return Ok(());
            }
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            circuit_gates[min].push(format!(
                "gategroup({}, replace_by_len, label: \"GetStateVector: {}\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                op.readout()
            ));
            let group_index = circuit_gates[min].len() - 1;
            let old_len = circuit_gates
                .iter()
                .map(|gates| effective_len(gates))
                .collect::<Vec<usize>>();
            for operation in circuit.iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    operation,
                    render_pragmas,
                )?;
            }
            let max_gates_len_diff = qubits
                .iter()
                .map(|&qubit| effective_len(&circuit_gates[qubit]) - old_len[qubit])
                .max()
                .unwrap_or(0);
            circuit_gates[min][group_index] = circuit_gates[min][group_index]
                .replace("replace_by_len", &max_gates_len_diff.to_string());
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::PragmaGetDensityMatrix(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let circuit = op.circuit().clone().unwrap_or(
                (0..circuit_gates.len())
                    .map(|qubit| Operation::from(Identity::new(qubit)))
                    .collect(),
            );
            let mut used_qubits: Vec<usize> = Vec::new();
            match circuit.involved_qubits() {
                InvolvedQubits::Set(involved_qubits) => {
                    for qubit in involved_qubits.iter() {
                        if !used_qubits.contains(qubit) {
                            used_qubits.push(*qubit);
                        }
                    }
                }
                InvolvedQubits::All => {
                    for qubit in 0..circuit_gates.len() {
                        if !used_qubits.contains(&qubit) {
                            used_qubits.push(qubit);
                        }
                    }
                }
                InvolvedQubits::None => {}
            }
            if used_qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = used_qubits.iter().min().unwrap().to_owned();
            let max = used_qubits.iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            if circuit.is_empty() || qubits.is_empty() {
                return Ok(());
            }
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            circuit_gates[min].push(format!(
                "gategroup({}, replace_by_len, label: \"GetDensityMatrix: {}\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                op.readout(),
            ));
            let group_index = circuit_gates[min].len() - 1;
            let old_len = circuit_gates
                .iter()
                .map(|gates| effective_len(gates))
                .collect::<Vec<usize>>();
            for operation in circuit.iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    operation,
                    render_pragmas,
                )?;
            }
            let max_gates_len_diff = qubits
                .iter()
                .map(|&qubit| effective_len(&circuit_gates[qubit]) - old_len[qubit])
                .max()
                .unwrap_or(0);
            circuit_gates[min][group_index] = circuit_gates[min][group_index]
                .replace("replace_by_len", &max_gates_len_diff.to_string());
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::PragmaGetOccupationProbability(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let circuit = op.circuit().clone().unwrap_or(
                (0..circuit_gates.len())
                    .map(|qubit| Operation::from(Identity::new(qubit)))
                    .collect(),
            );
            let mut used_qubits: Vec<usize> = Vec::new();
            match circuit.involved_qubits() {
                InvolvedQubits::Set(involved_qubits) => {
                    for qubit in involved_qubits.iter() {
                        if !used_qubits.contains(qubit) {
                            used_qubits.push(*qubit);
                        }
                    }
                }
                InvolvedQubits::All => {
                    for qubit in 0..circuit_gates.len() {
                        if !used_qubits.contains(&qubit) {
                            used_qubits.push(qubit);
                        }
                    }
                }
                InvolvedQubits::None => {}
            }
            if used_qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = used_qubits.iter().min().unwrap().to_owned();
            let max = used_qubits.iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            if circuit.is_empty() || qubits.is_empty() {
                return Ok(());
            }
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            circuit_gates[min].push(format!(
                "gategroup({}, replace_by_len, label: \"GetOccupationProbability: {}\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                op.readout(),
            ));
            let group_index = circuit_gates[min].len() - 1;
            let old_len = circuit_gates
                .iter()
                .map(|gates| effective_len(gates))
                .collect::<Vec<usize>>();
            for operation in circuit.iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    operation,
                    render_pragmas,
                )?;
            }
            let max_gates_len_diff = qubits
                .iter()
                .map(|&qubit| effective_len(&circuit_gates[qubit]) - old_len[qubit])
                .max()
                .unwrap_or(0);
            circuit_gates[min][group_index] = circuit_gates[min][group_index]
                .replace("replace_by_len", &max_gates_len_diff.to_string());
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::PragmaGetPauliProduct(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let mut circuit = op.circuit().clone();
            for (&qubit, op_val) in op.qubit_paulis() {
                match op_val {
                    0 => circuit.add_operation(Identity::new(qubit)),
                    1 => circuit.add_operation(PauliX::new(qubit)),
                    2 => circuit.add_operation(PauliY::new(qubit)),
                    3 => circuit.add_operation(PauliZ::new(qubit)),
                    _ => {
                        return Err(RoqoqoBackendError::RoqoqoError(
                            RoqoqoError::QubitMappingError { qubit },
                        ))
                    }
                }
            }
            let mut used_qubits: Vec<usize> = Vec::new();
            match circuit.involved_qubits() {
                InvolvedQubits::Set(involved_qubits) => {
                    for qubit in involved_qubits.iter() {
                        if !used_qubits.contains(qubit) {
                            used_qubits.push(*qubit);
                        }
                    }
                }
                InvolvedQubits::All => {
                    for qubit in 0..circuit_gates.len() {
                        if !used_qubits.contains(&qubit) {
                            used_qubits.push(qubit);
                        }
                    }
                }
                InvolvedQubits::None => {}
            }
            if used_qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = used_qubits.iter().min().unwrap().to_owned();
            let max = used_qubits.iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            if circuit.is_empty() || qubits.is_empty() {
                return Ok(());
            }
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            circuit_gates[min].push(format!(
                "gategroup({}, replace_by_len, label: \"GetPauliProduct: {}\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                op.readout(),
            ));
            let group_index = circuit_gates[min].len() - 1;
            let old_len = circuit_gates
                .iter()
                .map(|gates| effective_len(gates))
                .collect::<Vec<usize>>();
            for operation in circuit.iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    operation,
                    render_pragmas,
                )?;
            }
            let max_gates_len_diff = qubits
                .iter()
                .map(|&qubit| effective_len(&circuit_gates[qubit]) - old_len[qubit])
                .max()
                .unwrap_or(0);
            circuit_gates[min][group_index] = circuit_gates[min][group_index]
                .replace("replace_by_len", &max_gates_len_diff.to_string());
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::PragmaRepeatedMeasurement(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let used_qubits: Vec<usize> = op
                .qubit_mapping()
                .clone()
                .map_or((0..circuit_gates.len()).collect(), |map| {
                    map.keys().cloned().collect()
                });
            if used_qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = used_qubits.iter().min().unwrap().to_owned();
            let max = used_qubits.iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            circuit_gates[min].push(format!(
                "gategroup({}, 1, label: \"Repeat {} times\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                op.number_measurements(),
            ));
            for &qubit in used_qubits.iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    &Operation::from(MeasureQubit::new(qubit, "ro".to_owned(), qubit)),
                    render_pragmas,
                )?;
            }
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::InputSymbolic(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let n_qubits = circuit_gates.len();
            flatten_qubits(circuit_gates, &(0..n_qubits).collect::<Vec<usize>>());
            circuit_gates[0].push(format!(
                "slice(label: $ \"Replace Symbole:\"\\ {}=>{} $)",
                op.name(),
                format_calculator(&CalculatorFloat::from(op.input())),
            ));
            Ok(())
        }
        Operation::PragmaLoop(op) => {
            if op.circuit().is_empty() {
                return Ok(());
            }
            prepare_for_slice(circuit_gates, circuit_lock);
            let mut used_qubits: Vec<usize> = Vec::new();
            match op.involved_qubits() {
                InvolvedQubits::Set(involved_qubits) => {
                    for qubit in involved_qubits.iter() {
                        if !used_qubits.contains(qubit) {
                            used_qubits.push(*qubit);
                        }
                    }
                }
                InvolvedQubits::All => {
                    for qubit in 0..circuit_gates.len() {
                        if !used_qubits.contains(&qubit) {
                            used_qubits.push(qubit);
                        }
                    }
                }
                InvolvedQubits::None => {}
            }
            let min = used_qubits.iter().min().unwrap_or(&0_usize).to_owned();
            let max = used_qubits.iter().max().unwrap_or(&0_usize).to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            if qubits.is_empty() {
                return Ok(());
            }
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            circuit_gates[min].push(format!(
                "gategroup({}, replace_by_len, label: \"Loop: {} times\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                match op.repetitions() {
                    CalculatorFloat::Float(float_value) => (float_value.floor() as usize).to_string(),
                    _ => format_calculator(op.repetitions()).replace('"', "")
                }
            ));
            let group_index = circuit_gates[min].len() - 1;
            let old_len = circuit_gates
                .iter()
                .map(|gates| effective_len(gates))
                .collect::<Vec<usize>>();
            for operation in op.circuit().iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    operation,
                    render_pragmas,
                )?;
            }
            let max_gates_len_diff = qubits
                .iter()
                .map(|&qubit| effective_len(&circuit_gates[qubit]) - old_len[qubit])
                .max()
                .unwrap_or(0);
            circuit_gates[min][group_index] = circuit_gates[min][group_index]
                .replace("replace_by_len", &max_gates_len_diff.to_string());
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::PhaseShiftedControlledPhase(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"PhaseShiftControlledPhase\"({},{})$)",
                format_calculator(op.theta()),
                format_calculator(op.phi())
            ));
            Ok(())
        }
        Operation::ControlledRotateX(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"Rx\"({}) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::ControlledRotateXY(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"Rxy\"({},{}) $)",
                format_calculator(op.theta()),
                format_calculator(op.phi()),
            ));
            Ok(())
        }
        Operation::ControlledControlledPauliZ(op) => {
            let qubits = &[*op.control_0(), *op.target(), *op.control_1()];
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            flatten_qubits(circuit_gates, qubits);
            circuit_gates[*op.control_0()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_0() as i32
            ));
            circuit_gates[*op.control_1()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_1() as i32
            ));
            circuit_gates[*op.target()].push("gate($ Z $)".to_owned());
            Ok(())
        }
        Operation::ControlledControlledPhaseShift(op) => {
            let qubits = &[*op.control_0(), *op.target(), *op.control_1()];
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            flatten_qubits(circuit_gates, qubits);
            circuit_gates[*op.control_0()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_0() as i32
            ));
            circuit_gates[*op.control_1()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_1() as i32
            ));
            circuit_gates[*op.target()].push(format!("gate($ \"PhaseShift\"({}) $)", op.theta()));
            Ok(())
        }
        Operation::Toffoli(op) => {
            let qubits = &[*op.control_0(), *op.target(), *op.control_1()];
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            circuit_gates[*op.control_0()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_0() as i32
            ));
            circuit_gates[*op.control_1()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_1() as i32
            ));
            circuit_gates[*op.target()].push("targ()".to_owned());
            Ok(())
        }
        Operation::GPi(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"GPi\"({}) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::GPi2(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push(format!(
                "gate($ \"GPi2\"({}) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::PragmaControlledCircuit(op) => {
            if op.circuit().is_empty() {
                return Ok(());
            }
            prepare_for_slice(circuit_gates, circuit_lock);
            let mut used_qubits: Vec<usize> = Vec::new();
            match op.circuit().involved_qubits() {
                InvolvedQubits::Set(involved_qubits) => {
                    for qubit in involved_qubits.iter() {
                        if !used_qubits.contains(qubit) {
                            used_qubits.push(*qubit);
                        }
                    }
                }
                InvolvedQubits::All => {
                    for qubit in 0..circuit_gates.len() {
                        if !used_qubits.contains(&qubit) {
                            used_qubits.push(qubit);
                        }
                    }
                }
                InvolvedQubits::None => {}
            }
            if used_qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = used_qubits.iter().min().unwrap().to_owned();
            let max = used_qubits.iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            circuit_gates[min].push(format!(
                "gategroup({}, replace_by_len, label: \"ControlledCircuit by qubit: {}\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                op.controlling_qubit(),
            ));
            let group_index = circuit_gates[min].len() - 1;
            let old_len = circuit_gates
                .iter()
                .map(|gates| effective_len(gates))
                .collect::<Vec<usize>>();
            for operation in op.circuit().iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    operation,
                    render_pragmas,
                )?;
            }
            let max_gates_len_diff = qubits
                .iter()
                .map(|&qubit| effective_len(&circuit_gates[qubit]) - old_len[qubit])
                .max()
                .unwrap_or(0);
            circuit_gates[min][group_index] = circuit_gates[min][group_index]
                .replace("replace_by_len", &max_gates_len_diff.to_string());
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::Squeezing(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            bosonic_gates[*op.mode()].push(format!(
                "gate($ \"Squeezing\"({},{}) $)",
                format_calculator(op.squeezing()),
                format_calculator(op.phase()),
            ));
            Ok(())
        }
        Operation::PhaseShift(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            bosonic_gates[*op.mode()].push(format!(
                "gate($ \"PhaseShift\"({}) $)",
                format_calculator(op.phase()),
            ));
            Ok(())
        }
        Operation::BeamSplitter(op) => {
            let min = *op.mode_0().min(op.mode_1());
            let max = *op.mode_0().max(op.mode_1());
            let modes: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(bosonic_gates, &modes);
            for &mode in modes.iter() {
                prepare_for_bosonic(mode, bosonic_gates, bosonic_lock);
            }
            flatten_qubits(bosonic_gates, &modes);
            bosonic_gates[min].push(format!(
                "mqgate($ \"BeamSplitter\"\\ ({},{}) $, n: {}, width: 9em, inputs: ((qubit: {}), (qubit: {})))",
                format_calculator(op.theta()),
                format_calculator(op.phi()),
                modes.len(),
                format_qubit_input(*op.mode_0() - min, "x"),
                format_qubit_input(*op.mode_1() - min, "x")
            ));
            for gates in circuit_gates.iter_mut().take(max + 1).skip(min + 1) {
                gates.push("1".to_owned());
            }
            Ok(())
        }
        Operation::PhotonDetection(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            bosonic_gates[*op.mode()].push("meter()".to_owned());
            Ok(())
        }
        Operation::Identity(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ I $".to_owned());
            Ok(())
        }
        Operation::PragmaAnnotatedOp(op) => {
            prepare_for_slice(circuit_gates, circuit_lock);
            let mut used_qubits: Vec<usize> = Vec::new();
            match op.involved_qubits() {
                InvolvedQubits::Set(involved_qubits) => {
                    for qubit in involved_qubits.iter() {
                        if !used_qubits.contains(qubit) {
                            used_qubits.push(*qubit);
                        }
                    }
                }
                InvolvedQubits::All => {
                    for qubit in 0..circuit_gates.len() {
                        if !used_qubits.contains(&qubit) {
                            used_qubits.push(qubit);
                        }
                    }
                }
                InvolvedQubits::None => {}
            }
            if used_qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = used_qubits.iter().min().unwrap().to_owned();
            let max = used_qubits.iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            circuit_gates[min].push(format!(
                "gategroup({}, 1, label: \"{}\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                op.annotation,
            ));
            add_gate(
                circuit_gates,
                bosonic_gates,
                classical_gates,
                circuit_lock,
                bosonic_lock,
                classical_lock,
                &op.operation,
                render_pragmas,
            )?;
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::EchoCrossResonance(op) => {
            prepare_for_ctrl(circuit_gates, circuit_lock, &[*op.control(), *op.target()]);
            circuit_gates[*op.control()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control() as i32
            ));
            circuit_gates[*op.target()].push("gate($ \"EchoCrossResonance\" $)".to_owned());
            Ok(())
        }
        Operation::PhaseDisplacement(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            bosonic_gates[*op.mode()].push(format!(
                "gate($ \"PhaseDisplacement\"({},{}) $)",
                format_calculator(op.displacement()),
                format_calculator(op.phase()),
            ));
            Ok(())
        }
        #[cfg(feature = "unstable_operation_definition")]
        Operation::CallDefinedGate(op) => {
            if op.qubits().is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = op.qubits().iter().min().unwrap().to_owned();
            let max = op.qubits().iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubits);
            circuit_gates[min].push(format!(
                r#"mqgate($ "CallDefinedGate\n\"{}\"" $, n: {}, width: 11em, inputs: ({}))"#,
                op.gate_name(),
                qubits.len(),
                op.qubits()
                    .iter()
                    .map(|qubit| format!("(qubit: {})", format_qubit_input(qubit - min, "x")))
                    .collect::<Vec<String>>()
                    .join(",")
            ));
            push_ones(circuit_gates, min, max);
            Ok(())
        }
        #[cfg(feature = "unstable_operation_definition")]
        Operation::GateDefinition(op) => {
            if op.circuit().is_empty() {
                return Ok(());
            }
            prepare_for_slice(circuit_gates, circuit_lock);
            let mut used_qubits: Vec<usize> = Vec::new();
            match op.circuit().involved_qubits() {
                InvolvedQubits::Set(involved_qubits) => {
                    for qubit in involved_qubits.iter() {
                        if !used_qubits.contains(qubit) {
                            used_qubits.push(*qubit);
                        }
                    }
                }
                InvolvedQubits::All => {
                    for qubit in 0..circuit_gates.len() {
                        if !used_qubits.contains(&qubit) {
                            used_qubits.push(qubit);
                        }
                    }
                }
                InvolvedQubits::None => {}
            }
            if used_qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {op:?}"),
                });
            }
            let min = used_qubits.iter().min().unwrap().to_owned();
            let max = used_qubits.iter().max().unwrap().to_owned();
            let qubits: Vec<usize> = (min..max + 1).collect();
            add_qubits_vec(circuit_gates, &qubits);
            flatten_qubits(circuit_gates, &qubits);
            circuit_gates[min].push(format!(
                "gategroup({}, replace_by_len, label: \"GateDefinition: {}\",  stroke: (dash: \"dotted\"))",
                qubit_range(&qubits),
                op.name(),
            ));
            let group_index = circuit_gates[min].len() - 1;
            let old_len = circuit_gates
                .iter()
                .map(|gates| effective_len(gates))
                .collect::<Vec<usize>>();
            for operation in op.circuit().iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    operation,
                    render_pragmas,
                )?;
            }
            let max_gates_len_diff = qubits
                .iter()
                .map(|&qubit| effective_len(&circuit_gates[qubit]) - old_len[qubit])
                .max()
                .unwrap_or(0);
            circuit_gates[min][group_index] = circuit_gates[min][group_index]
                .replace("replace_by_len", &max_gates_len_diff.to_string());
            flatten_qubits(circuit_gates, &qubit_range_vec(&qubits));
            Ok(())
        }
        Operation::QuantumRabi(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            for qubit in *op.qubit() + 1..circuit_gates.len() + 10 {
                circuit_lock.push((qubit, effective_len(&circuit_gates[*op.qubit()])));
            }
            for mode in 0..*op.mode() {
                bosonic_lock.push((mode, effective_len(&bosonic_gates[*op.mode()])));
            }
            circuit_gates[*op.qubit()].push(format!(
                "mqgate($ {} * X $, extent: 1.4em, target: replace_by_n_qubits_plus_{}-{})",
                format_calculator(op.theta()),
                *op.mode(),
                *op.qubit(),
            ));
            bosonic_gates[*op.mode()].push(format!(
                "gate($ {}*(b^(dagger)+b) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::LongitudinalCoupling(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            for qubit in *op.qubit() + 1..circuit_gates.len() + 10 {
                circuit_lock.push((qubit, effective_len(&circuit_gates[*op.qubit()])));
            }
            for mode in 0..*op.mode() {
                bosonic_lock.push((mode, effective_len(&bosonic_gates[*op.mode()])));
            }
            circuit_gates[*op.qubit()].push(format!(
                "mqgate($ {} * Z $, extent: 1.4em, target: replace_by_n_qubits_plus_{}-{})",
                format_calculator(op.theta()),
                *op.mode(),
                *op.qubit(),
            ));
            bosonic_gates[*op.mode()].push(format!(
                "gate($ {}*(b^(dagger)+b) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::JaynesCummings(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            for qubit in *op.qubit() + 1..circuit_gates.len() + 10 {
                circuit_lock.push((qubit, effective_len(&circuit_gates[*op.qubit()])));
            }
            for mode in 0..*op.mode() {
                bosonic_lock.push((mode, effective_len(&bosonic_gates[*op.mode()])));
            }
            circuit_gates[*op.qubit()].push(format!(
                "mqgate($ {} * (sigma^-+sigma^+) $, extent: 1.4em, target: replace_by_n_qubits_plus_{}-{})",
                format_calculator(op.theta()),
                *op.mode(),
                *op.qubit(),
            ));
            bosonic_gates[*op.mode()].push(format!(
                "gate($ {}*(b^(dagger)+b) $)",
                format_calculator(op.theta())
            ));
            Ok(())
        }
        Operation::SingleExcitationStore(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            for qubit in *op.qubit() + 1..circuit_gates.len() + 10 {
                circuit_lock.push((qubit, effective_len(&circuit_gates[*op.qubit()])));
            }
            for mode in 0..*op.mode() {
                bosonic_lock.push((mode, effective_len(&bosonic_gates[*op.mode()])));
            }
            circuit_gates[*op.qubit()].push(format!(
                r#"mqgate($ alpha"|0>" + beta"|1>" -> "|0>" $, target: replace_by_n_qubits_plus_{}-{})"#,
                *op.mode(),
                *op.qubit(),
            ));
            bosonic_gates[*op.mode()]
                .push(r#"gate($ "|0>" -> alpha"|0>" + beta"|1>" $)"#.to_string());
            Ok(())
        }
        Operation::SingleExcitationLoad(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            for qubit in *op.qubit() + 1..circuit_gates.len() + 10 {
                circuit_lock.push((qubit, effective_len(&circuit_gates[*op.qubit()])));
            }
            for mode in 0..*op.mode() {
                bosonic_lock.push((mode, effective_len(&bosonic_gates[*op.mode()])));
            }
            circuit_gates[*op.qubit()].push(format!(
                r#"mqgate($ "|0>" -> alpha"|0>" + beta"|1>" $, target: replace_by_n_qubits_plus_{}-{})"#,
                *op.mode(),
                *op.qubit(),
            ));
            bosonic_gates[*op.mode()]
                .push(r#"gate($ alpha"|0>" + beta"|1>" -> "|0>" $)"#.to_string());
            Ok(())
        }
        Operation::CZQubitResonator(op) => {
            add_qubits_vec(bosonic_gates, &[*op.mode()]);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            prepare_for_bosonic(*op.mode(), bosonic_gates, bosonic_lock);
            flatten_multiple_vec(circuit_gates, bosonic_gates, &[*op.qubit()], &[*op.mode()]);
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            for qubit in *op.qubit() + 1..circuit_gates.len() {
                if circuit_gates.len() > qubit
                    && effective_len(&circuit_gates[qubit])
                        > effective_len(&circuit_gates[*op.qubit()])
                {
                    flatten_qubits(circuit_gates, &[*op.qubit(), qubit]);
                }
            }
            for boson in 0..*op.mode() {
                if bosonic_gates.len() > boson
                    && effective_len(&bosonic_gates[boson])
                        > effective_len(&bosonic_gates[*op.mode()])
                {
                    flatten_qubits(bosonic_gates, &[*op.mode(), boson]);
                }
            }
            for qubit in *op.qubit() + 1..circuit_gates.len() + 10 {
                circuit_lock.push((qubit, effective_len(&circuit_gates[*op.qubit()])));
            }
            for mode in 0..*op.mode() {
                bosonic_lock.push((mode, effective_len(&bosonic_gates[*op.mode()])));
            }
            circuit_gates[*op.qubit()].push(format!(
                r#"ctrl(replace_by_n_qubits_plus_{}-{})"#,
                *op.mode(),
                *op.qubit(),
            ));
            bosonic_gates[*op.mode()].push(r#"gate($ Z $)"#.to_string());
            Ok(())
        }
        Operation::DefinitionBit(op) => {
            classical_gates.push(Vec::new());
            let index = classical_gates.len() - 1;
            classical_gates[index].push(format!("lstick($ \"{} : \" $)", op.name()));
            classical_gates[index].push(format!("setwire({})", 2));
            Ok(())
        }
        Operation::InputBit(op) => {
            if let Some((index, _)) = classical_gates
                .iter()
                .enumerate()
                .find(|(_ind, register)| register[0].contains(op.name()))
            {
                classical_gates[index].push(format!(
                    "gate($ \"InputBit:\"\\ {}=>#{} $)",
                    op.index(),
                    op.value()
                ));
            }
            Ok(())
        }
        Operation::SqrtPauliY(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ sqrt(Y) $".to_owned());
            Ok(())
        }
        Operation::InvSqrtPauliY(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ sqrt(Y)^(dagger) $".to_owned());
            Ok(())
        }
        Operation::InvSGate(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ S^(dagger) $".to_owned());
            Ok(())
        }
        Operation::InvTGate(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ T^(dagger) $".to_owned());
            Ok(())
        }
        Operation::SXGate(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ \"SX\" $".to_owned());
            Ok(())
        }
        Operation::InvSXGate(op) => {
            add_qubits_vec(circuit_gates, &[*op.qubit()]);
            circuit_gates[*op.qubit()].push("$ \"SX\"^(dagger) $".to_owned());
            Ok(())
        }
        Operation::ControlledSWAP(op) => {
            let qubits = &[*op.control_0(), *op.target_0(), *op.target_1()];
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            let min = op.target_0().min(op.target_1()).to_owned();
            let max = op.target_0().max(op.target_1()).to_owned();
            circuit_gates[*op.control_0()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_0() as i32
            ));
            circuit_gates[min].push(format!("swap({})", max - min));
            circuit_gates[max].push("targX()".to_owned());
            Ok(())
        }
        Operation::PhaseShiftedControlledControlledZ(op) => {
            let qubits = &[*op.control_0(), *op.target(), *op.control_1()];
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            circuit_gates[*op.control_0()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_0() as i32
            ));
            circuit_gates[*op.control_1()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_1() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"PhaseShiftedControlledControlledZ\"({})$)",
                format_calculator(op.phi())
            ));
            Ok(())
        }
        Operation::PhaseShiftedControlledControlledPhase(op) => {
            let qubits = &[*op.control_0(), *op.target(), *op.control_1()];
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            circuit_gates[*op.control_0()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_0() as i32
            ));
            circuit_gates[*op.control_1()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_1() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"PhaseShiftedControlledControlledPhase\"({},{})$)",
                format_calculator(op.theta()),
                format_calculator(op.phi())
            ));
            Ok(())
        }
        Operation::TripleControlledPauliX(op) => {
            let qubits = &[
                *op.control_0(),
                *op.control_1(),
                *op.control_2(),
                *op.target(),
            ];
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            circuit_gates[*op.control_0()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_0() as i32
            ));
            circuit_gates[*op.control_1()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_1() as i32
            ));
            circuit_gates[*op.control_2()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_2() as i32
            ));
            circuit_gates[*op.target()].push("targ()".to_owned());
            Ok(())
        }
        Operation::TripleControlledPauliZ(op) => {
            let qubits = &[
                *op.control_0(),
                *op.control_1(),
                *op.control_2(),
                *op.target(),
            ];
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            circuit_gates[*op.control_0()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_0() as i32
            ));
            circuit_gates[*op.control_1()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_1() as i32
            ));
            circuit_gates[*op.control_2()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_2() as i32
            ));
            circuit_gates[*op.target()].push("$ Z $".to_owned());
            Ok(())
        }
        Operation::TripleControlledPhaseShift(op) => {
            let qubits = &[
                *op.control_0(),
                *op.control_1(),
                *op.control_2(),
                *op.target(),
            ];
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            circuit_gates[*op.control_0()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_0() as i32
            ));
            circuit_gates[*op.control_1()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_1() as i32
            ));
            circuit_gates[*op.control_2()].push(format!(
                "ctrl({})",
                *op.target() as i32 - *op.control_2() as i32
            ));
            circuit_gates[*op.target()].push(format!(
                "gate($ \"TripleControlledPhaseShift\"({})$)",
                format_calculator(op.theta()),
            ));
            Ok(())
        }
        Operation::MultiQubitCNOT(multi_qubit_cnot) => {
            let qubits = multi_qubit_cnot.qubits();
            if qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {multi_qubit_cnot:?}"),
                });
            }
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, qubits);
            prepare_for_ctrl(circuit_gates, circuit_lock, qubits);
            for i in 0..qubits.len() - 1 {
                circuit_gates[qubits[i]]
                    .push(format!("ctrl({})", qubits[i + 1] as i32 - qubits[i] as i32));
            }
            circuit_gates[*qubits.last().unwrap()].push("targ()".to_owned());
            Ok(())
        }
        Operation::QFT(qft) => {
            let qubits = qft.qubits();
            if qubits.is_empty() {
                return Err(RoqoqoBackendError::GenericError {
                    msg: format!("Operations with no qubit in the input: {qft:?}"),
                });
            }
            add_qubits_vec(circuit_gates, qubits);
            flatten_qubits(circuit_gates, &qubit_range_vec(qubits));
            prepare_for_slice(circuit_gates, circuit_lock);
            circuit_gates[qubits[0]].push(format!(
                "gategroup({}, replace_by_len, label: \"QFT\",  stroke: (dash: \"dotted\"))",
                qubit_range(qubits),
            ));
            let group_index = circuit_gates[qubits[0]].len() - 1;
            let old_len = circuit_gates
                .iter()
                .map(|gates| effective_len(gates))
                .collect::<Vec<usize>>();
            for operation in qft.circuit().iter() {
                add_gate(
                    circuit_gates,
                    bosonic_gates,
                    classical_gates,
                    circuit_lock,
                    bosonic_lock,
                    classical_lock,
                    operation,
                    render_pragmas,
                )?;
            }
            let max_gates_len_diff = qubits
                .iter()
                .map(|&qubit| effective_len(&circuit_gates[qubit]) - old_len[qubit])
                .max()
                .unwrap_or(0);
            circuit_gates[qubits[0]][group_index] = circuit_gates[qubits[0]][group_index]
                .replace("replace_by_len", &max_gates_len_diff.to_string());
            flatten_qubits(circuit_gates, &qubit_range_vec(qubits));
            Ok(())
        }
        _ => {
            if ALLOWED_OPERATIONS.contains(&operation.hqslang()) {
                Ok(())
            } else {
                Err(RoqoqoBackendError::OperationNotInBackend {
                    backend: "TypstBackend",
                    hqslang: operation.hqslang(),
                })
            }
        }
    }
}
