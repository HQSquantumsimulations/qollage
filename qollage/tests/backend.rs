// Copyright © 2022-2024 HQS Quantum Simulations GmbH. All Rights Reserved.
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
//! Testing the qollage Backend

use std::{fs, path::Path};

use pyo3::{types::PyAnyMethods, Bound, Python};
use qollage::{circuit_to_typst_str, draw_circuit, save_circuit};
use qoqo::{operations::convert_operation_to_pyobject, CircuitWrapper};
use qoqo_calculator::CalculatorFloat;
use roqoqo::{operations::*, Circuit};

// helper functions
fn circuitpy_from_circuitru(py: Python, circuit: Circuit) -> Bound<CircuitWrapper> {
    let circuit_type = py.get_type_bound::<CircuitWrapper>();
    let binding = circuit_type.call0().unwrap();
    let circuitpy = binding.downcast::<CircuitWrapper>().unwrap();
    for op in circuit {
        let new_op = convert_operation_to_pyobject(op).unwrap();
        circuitpy.call_method1("add", (new_op.clone(),)).unwrap();
    }
    circuitpy.to_owned()
}

#[test]
fn test_file() {
    let mut circuit = Circuit::new();
    circuit.add_operation(Hadamard::new(0));
    circuit.add_operation(RotateY::new(1, CalculatorFloat::PI));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("sys_2 + 2*1e0")));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("theta.alt")));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("times.three.r")));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("test")));
    circuit.add_operation(PragmaBoostNoise::new(CalculatorFloat::from("12.7")));
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

    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let circuitpy = circuitpy_from_circuitru(py, circuit);

        save_circuit(&circuitpy, None, 3.5, "PragmaOverrotation", None).unwrap();
    });
    let read_in_path = Path::new("circuit.png");
    assert!(read_in_path.exists());
    fs::remove_file(read_in_path).unwrap();
}

#[test]
fn test_str() {
    let mut circuit = Circuit::new();
    circuit.add_operation(Hadamard::new(0));
    circuit.add_operation(RotateY::new(1, CalculatorFloat::PI));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("sys_2 + 2*1e0")));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("theta.alt")));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("times.three.r")));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("test")));
    circuit.add_operation(PragmaBoostNoise::new(CalculatorFloat::from("12.7")));
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

    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let circuitpy = circuitpy_from_circuitru(py, circuit);

        circuit_to_typst_str(&circuitpy, "PragmaOverrotation", None).unwrap();
    });
}

#[test]
fn test_draw() {
    let mut circuit = Circuit::new();
    circuit.add_operation(Hadamard::new(0));
    circuit.add_operation(RotateY::new(1, CalculatorFloat::PI));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("sys_2 + 2*1e0")));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("theta.alt")));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("times.three.r")));
    circuit.add_operation(RotateX::new(0, CalculatorFloat::from("test")));
    circuit.add_operation(PragmaBoostNoise::new(CalculatorFloat::from("12.7")));
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

    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let circuitpy = circuitpy_from_circuitru(py, circuit);

        draw_circuit(&circuitpy, 0.5, "PragmaOverrotation", None).unwrap();
    });
}
