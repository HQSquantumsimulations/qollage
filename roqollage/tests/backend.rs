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
//
//! Testing the roqollage backend

use std::f32::consts::PI;

use qoqo_calculator::CalculatorFloat;
use roqollage::{
    circuit_into_typst_str, circuit_to_image, InitializationMode, RenderPragmas, TypstBackend,
};
use roqoqo::{operations::*, Circuit};
use serial_test::serial;
use typst::eval::Tracer;

#[test]
#[serial]
fn test_str() {
    let mut circuit = Circuit::new();
    circuit.add_operation(Hadamard::new(0));
    circuit.add_operation(RotateY::new(1, -CalculatorFloat::FRAC_1_SQRT_2));
    circuit.add_operation(RotateX::new(0, -CalculatorFloat::FRAC_PI_2));
    circuit.add_operation(RotateX::new(0, -CalculatorFloat::FRAC_PI_4));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::SQRT_2));
    circuit.add_operation(RotateX::new(0, -CalculatorFloat::SQRT_2));
    circuit.add_operation(RotateX::new(0, -CalculatorFloat::from("(gamma)")));
    circuit.add_operation(RotateX::new(0, -CalculatorFloat::from("(gamma))")));
    circuit.add_operation(PragmaBoostNoise::new(CalculatorFloat::from("(theta)")));
    circuit.add_operation(CNOT::new(2, 5));
    circuit.add_operation(PragmaOverrotation::new(
        "Hadamard".to_owned(),
        vec![3, 4, 2],
        0.69,
        0.2,
    ));
    circuit.add_operation(Hadamard::new(3));
    circuit.add_operation(CNOT::new(0, 1));
    circuit.add_operation(SWAP::new(2, 1));
    circuit.add_operation(Toffoli::new(0, 1, 4));

    circuit_into_typst_str(&circuit, RenderPragmas::None, None).unwrap();
    circuit_into_typst_str(
        &circuit,
        RenderPragmas::None,
        Some(InitializationMode::Qubit),
    )
    .unwrap();
}

#[test]
#[serial]
fn test_image() {
    let mut circuit = Circuit::new();
    let mut loop_circuit = Circuit::new();

    loop_circuit.add_operation(Hadamard::new(0));
    loop_circuit.add_operation(CNOT::new(0, 1));
    loop_circuit.add_operation(CNOT::new(1, 2));
    loop_circuit.add_operation(Hadamard::new(2));

    circuit.add_operation(DefinitionBit::new("ro".to_owned(), 2, true));
    circuit.add_operation(Hadamard::new(0));
    circuit.add_operation(RotateY::new(1, -CalculatorFloat::FRAC_1_SQRT_2));
    circuit.add_operation(CNOT::new(2, 5));
    circuit.add_operation(PragmaOverrotation::new(
        "Hadamard".to_owned(),
        vec![3, 4, 2],
        0.69,
        0.2,
    ));
    circuit.add_operation(Hadamard::new(3));
    circuit.add_operation(CNOT::new(0, 3));
    circuit.add_operation(SWAP::new(2, 1));
    circuit.add_operation(Toffoli::new(0, 1, 4));
    circuit.add_operation(PragmaLoop::new(CalculatorFloat::from("4.0"), loop_circuit));
    circuit.add_operation(InputSymbolic::new("theta".to_owned(), 0.23));
    circuit.add_operation(RotateX::new(0, -CalculatorFloat::from("theta")));
    circuit.add_operation(BeamSplitter::new(
        0,
        1,
        CalculatorFloat::PI,
        CalculatorFloat::from("theta"),
    ));
    circuit.add_operation(PhaseDisplacement::new(
        3,
        CalculatorFloat::FRAC_PI_4,
        -CalculatorFloat::FRAC_PI_4,
    ));
    circuit.add_operation(QuantumRabi::new(2, 1, CalculatorFloat::ONE));

    circuit.add_operation(MeasureQubit::new(0, "ro".to_owned(), 0));
    circuit.add_operation(MeasureQubit::new(1, "ro".to_owned(), 1));

    circuit_to_image(
        &circuit,
        None,
        RenderPragmas::None,
        Some(InitializationMode::State),
    )
    .unwrap();
    circuit_to_image(
        &circuit,
        None,
        RenderPragmas::None,
        Some(InitializationMode::Qubit),
    )
    .unwrap();
}

#[test]
#[serial]
fn test_flatten() {
    let mut circuit = Circuit::new();
    circuit.add_operation(DefinitionBit::new("ro".to_owned(), 2, true));
    circuit.add_operation(DefinitionBit::new("ro2".to_owned(), 2, true));
    circuit.add_operation(Hadamard::new(0));
    circuit.add_operation(RotateY::new(0, -CalculatorFloat::FRAC_1_SQRT_2));
    circuit.add_operation(RotateY::new(1, -CalculatorFloat::FRAC_1_SQRT_2));
    circuit.add_operation(MeasureQubit::new(0, "ro".to_owned(), 1));
    circuit.add_operation(InputBit::new("ro".to_owned(), 0, true));
    circuit.add_operation(InputBit::new("ro".to_owned(), 0, true));
    circuit.add_operation(MeasureQubit::new(1, "ro2".to_owned(), 0));

    circuit_to_image(
        &circuit,
        None,
        RenderPragmas::None,
        Some(InitializationMode::State),
    )
    .unwrap();
}

#[test]
#[serial]
fn test_flatten_boson() {
    let mut circuit = Circuit::new();
    circuit.add_operation(DefinitionBit::new("ro".to_owned(), 2, true));
    circuit.add_operation(Hadamard::new(0));
    circuit.add_operation(RotateY::new(1, -CalculatorFloat::FRAC_1_SQRT_2));
    circuit.add_operation(RotateY::new(1, -CalculatorFloat::FRAC_1_SQRT_2));
    circuit.add_operation(Squeezing::new(
        0,
        CalculatorFloat::ZERO,
        CalculatorFloat::ZERO,
    ));
    circuit.add_operation(Squeezing::new(
        0,
        CalculatorFloat::ZERO,
        CalculatorFloat::ZERO,
    ));
    circuit.add_operation(Squeezing::new(
        0,
        CalculatorFloat::ZERO,
        CalculatorFloat::ZERO,
    ));
    circuit.add_operation(MeasureQubit::new(0, "ro".to_owned(), 1));

    circuit_to_image(
        &circuit,
        None,
        RenderPragmas::None,
        Some(InitializationMode::State),
    )
    .unwrap();
}

#[test]
#[serial]
fn test_resonator() {
    let mut circuit = Circuit::new();
    circuit.add_operation(DefinitionBit::new("ro".to_owned(), 2, true));
    circuit.add_operation(Hadamard::new(0));
    circuit.add_operation(RotateY::new(1, -CalculatorFloat::FRAC_1_SQRT_2));
    circuit.add_operation(RotateY::new(3, -CalculatorFloat::FRAC_1_SQRT_2));
    circuit.add_operation(Squeezing::new(
        0,
        CalculatorFloat::ZERO,
        CalculatorFloat::ZERO,
    ));
    circuit.add_operation(CZQubitResonator::new(0, 2));

    circuit_to_image(
        &circuit,
        None,
        RenderPragmas::None,
        Some(InitializationMode::State),
    )
    .unwrap();
}

#[test]
#[serial]
fn test_values() {
    let mut circuit = Circuit::new();
    circuit.add_operation(RotateY::new(
        0,
        CalculatorFloat::Float((3.0 * PI / 2.0).into()),
    ));
    circuit.add_operation(RotateY::new(
        0,
        CalculatorFloat::Float((-3.0 * PI / 2.0).into()),
    ));
    circuit.add_operation(RotateY::new(1, CalculatorFloat::Float((PI / 3.0).into())));
    circuit.add_operation(RotateY::new(1, CalculatorFloat::Float((-PI / 3.0).into())));
    circuit.add_operation(RotateY::new(
        2,
        CalculatorFloat::Float((2.0 * PI / 3.0).into()),
    ));
    circuit.add_operation(RotateY::new(
        2,
        CalculatorFloat::Float((-2.0 * PI / 3.0).into()),
    ));
    circuit.add_operation(RotateY::new(
        3,
        CalculatorFloat::Float((3.0 * PI / 4.0).into()),
    ));
    circuit.add_operation(RotateY::new(
        3,
        CalculatorFloat::Float((-3.0 * PI / 4.0).into()),
    ));

    circuit_into_typst_str(&circuit, RenderPragmas::None, None).unwrap();
}

#[test]
#[serial]
fn test_backend_today() {
    let backend = TypstBackend::new("#datetime.today().display()".to_owned()).unwrap();
    let mut tracer = Tracer::default();
    let _doc = typst::compile(&backend, &mut tracer).unwrap();
}
