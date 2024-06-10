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

use std::{collections::HashMap, io::Cursor, path::PathBuf, str::FromStr};

use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
};
use qoqo::{convert_into_circuit, CircuitWrapper};
use roqollage::{circuit_into_typst_str, circuit_to_image, InitializationMode, RenderPragmas};
use roqoqo::{
    operations::{InvolveQubits, Operate, OperateTwoQubit, Operation},
    RoqoqoError,
};

/// Saves the qoqo circuit as a png image
///
/// Args:
///     circuit (Circuit): The qoqo circuit to be saved
///     path (String): The path to where the image should be saved
///     pixel_per_point (f32): The pixels per point ration of the image.  
///        The higher the value, the bigger the image will be but the longer it will take to render  
///      render_pragmas (bool): How to render Pragmas operations:  
///        `"all"` to render every pragmas.
///        `"none"` to not render any pragmas.
///        `"PragmaOperation1, PragmaOperation2"` to render only some pragmas.  
///     initialization_mode (String): What to display at the begginning of the circuit. "state" for "|0>" and  
///         "qubit" for "q[n]" State will be used if the parameter is not set.
///
/// Raises:
///     TypeError: Circuit conversion error
///     ValueError: Operation not supported
#[pyfunction]
#[pyo3(signature = (circuit, path=None, pixel_per_point=3.0, render_pragmas="all", initialization_mode=None))]
pub fn save_circuit(
    circuit: &Bound<PyAny>,
    path: Option<PathBuf>,
    pixel_per_point: f32,
    render_pragmas: &str,
    initialization_mode: Option<String>,
) -> PyResult<()> {
    let circuit = convert_into_circuit(circuit).map_err(|x| {
        PyTypeError::new_err(format!("Cannot convert python object to Circuit: {x:?}"))
    })?;
    let initialization_mode = initialization_mode
        .map(|mode: String| InitializationMode::from_str(mode.as_str()))
        .transpose()
        .map_err(|x| PyValueError::new_err(format!("Initialization mode not accepted: {x:?}")))?;
    let image = circuit_to_image(
        &circuit,
        Some(pixel_per_point),
        RenderPragmas::from_str(render_pragmas).map_err(|x| {
            PyValueError::new_err(format!(
                "Error: render_pragmas is not in a suitable format: {x:?}"
            ))
        })?,
        initialization_mode,
    )
    .map_err(|x| PyValueError::new_err(format!("Error during Circuit drawing: {x:?}")))?;
    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|x| {
            PyValueError::new_err(format!(
                "Error during the generation of the Png file: {x:?}"
            ))
        })?;

    let path = match path {
        Some(path) => {
            if path.is_dir() && path.exists() {
                format!("{}/circuit.png", path.to_str().unwrap_or("."))
            } else {
                let s = path.to_str().unwrap_or("circuit").to_owned();
                if s.ends_with(".png") {
                    s
                } else {
                    format!("{}.png", s)
                }
            }
        }
        None => "circuit.png".to_owned(),
    };
    image
        .save(path)
        .map_err(|x| PyValueError::new_err(format!("Error during image saving: {x:?}")))?;
    Ok(())
}

/// Displays the qoqo circuit as an image output
///
/// Args:
///     circuit (Circuit): The qoqo circuit to draw
///     pixel_per_point (Option<f32>): The pixels per point ration of the image.  
///        The higher the value, the bigger the image will be but the longer it will take to render  
///     render_pragmas (bool): How to render Pragmas operations:  
///        `"all"` to render every pragmas.
///        `"none"` to not render any pragmas.
///        `"PragmaOperation1, PragmaOperation2"` to render only some pragmas.  
///     initialization_mode (String): What to display at the begginning of the circuit. "state" for "|0>" and  
///         "qubit" for "q[n]" State will be used if the parameter is not set.
///
/// Raises:
///     TypeError: Circuit conversion error
///     ValueError: Operation not supported
#[pyfunction]
#[pyo3(signature = (circuit, pixel_per_point=3.0, render_pragmas="All", initialization_mode=None))]
pub fn draw_circuit(
    circuit: &Bound<PyAny>,
    pixel_per_point: f32,
    render_pragmas: &str,
    initialization_mode: Option<String>,
) -> PyResult<()> {
    let circuit = convert_into_circuit(circuit).map_err(|x| {
        PyTypeError::new_err(format!("Cannot convert python object to Circuit: {x:?}"))
    })?;
    let initialization_mode = initialization_mode
        .map(|mode: String| InitializationMode::from_str(mode.as_str()))
        .transpose()
        .map_err(|x| PyValueError::new_err(format!("Initialization mode not accepted: {x:?}")))?;
    let image = circuit_to_image(
        &circuit,
        Some(pixel_per_point),
        RenderPragmas::from_str(render_pragmas).unwrap(),
        initialization_mode,
    )
    .map_err(|x| PyValueError::new_err(format!("Error during Circuit drawing: {x:?}")))?;
    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|x| {
            PyValueError::new_err(format!(
                "Error during the generation of the Png file: {x:?}"
            ))
        })?;

    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let pil = PyModule::import_bound(py, "PIL.Image").unwrap();
        let io = PyModule::import_bound(py, "io").unwrap();
        let display = PyModule::import_bound(py, "IPython.display").unwrap();
        let builtins = PyModule::import_bound(py, "builtins").unwrap();

        let bytes_image_data = builtins
            .call_method1("bytes", (buffer.clone().into_inner(),))
            .unwrap();
        let bytes_io = io.call_method1("BytesIO", (bytes_image_data,)).unwrap();
        let image = pil.call_method1("open", (bytes_io,)).unwrap();

        display.call_method1("display", (image,)).unwrap();
    });
    Ok(())
}

/// Displays the qoqo circuit as an image output
///
/// Args:
///     circuit (Circuit): The qoqo circuit to draw
///     render_pragmas (bool): How to render Pragmas operations:  
///        `"all"` to render every pragmas.
///        `"none"` to not render any pragmas.
///        `"PragmaOperation1, PragmaOperation2"` to render only some pragmas.  
///     initialization_mode (String): What to display at the begginning of the circuit. "state" for "|0>" and  
///         "qubit" for "q[n]" State will be used if the parameter is not set.
///
/// Raises:
///     TypeError: Circuit conversion error
///     ValueError: Operation not supported
#[pyfunction]
#[pyo3(signature = (circuit, render_pragmas="All", initialization_mode=None))]
pub fn circuit_to_typst_str(
    circuit: &Bound<PyAny>,
    render_pragmas: &str,
    initialization_mode: Option<String>,
) -> PyResult<String> {
    let circuit = convert_into_circuit(circuit).map_err(|x| {
        PyTypeError::new_err(format!("Cannot convert python object to Circuit: {x:?}"))
    })?;
    let initialization_mode = initialization_mode
        .map(|mode: String| InitializationMode::from_str(mode.as_str()))
        .transpose()
        .map_err(|x| PyValueError::new_err(format!("Initialization mode not accepted: {x:?}")))?;
    circuit_into_typst_str(
        &circuit,
        RenderPragmas::from_str(render_pragmas).unwrap(),
        initialization_mode,
    )
    .map_err(|x| PyValueError::new_err(format!("Error during Circuit drawing: {x:?}")))
}

/// Displays the qoqo circuit as an image output
///
/// Args:
///     circuit (Circuit): The qoqo circuit to draw
///     render_pragmas (bool): How to render Pragmas operations:  
///        `"all"` to render every pragmas.
///        `"none"` to not render any pragmas.
///        `"PragmaOperation1, PragmaOperation2"` to render only some pragmas.  
///     initialization_mode (String): What to display at the begginning of the circuit. "state" for "|0>" and  
///         "qubit" for "q[n]" State will be used if the parameter is not set.
///
/// Raises:
///     TypeError: Circuit conversion error
///     ValueError: Operation not supported
#[pyfunction]
#[pyo3(signature = (circuit))]
pub fn remove_two_qubit_gates_identities(circuit: &Bound<PyAny>) -> PyResult<qoqo::CircuitWrapper> {
    let circuit = convert_into_circuit(circuit).map_err(|x| {
        PyTypeError::new_err(format!("Cannot convert python object to Circuit: {x:?}"))
    })?;
    Ok(CircuitWrapper {
        internal: remove_identities(circuit).map_err(|e| PyValueError::new_err(e.to_string()))?,
    })
}

fn remove_identities(circuit: roqoqo::Circuit) -> Result<roqoqo::Circuit, RoqoqoError> {
    const UNITARY_GATES: &[&str] = &["CNOT", "SWAP", "iSWAP", "ControlledPauliZ"];
    let mut last_gates: HashMap<(usize, usize), Option<(String, Operation)>> =
        std::collections::HashMap::new();
    let mut new_circuit = roqoqo::Circuit::new();
    for op in circuit.iter() {
        if op.tags().contains(&"TwoQubitGateOperation")
            && UNITARY_GATES.contains(op.tags().last().unwrap_or(&""))
        {
            let qubits = match op {
                Operation::CNOT(cnot) => (*cnot.control(), *cnot.target()),
                Operation::SWAP(swap) => (*swap.control(), *swap.target()),
                Operation::ISwap(iswap) => (*iswap.control(), *iswap.target()),
                Operation::ControlledPauliZ(cpz) => (*cpz.control(), *cpz.target()),
                _ => {
                    return Err(RoqoqoError::GenericError {
                        msg: "Invalid operation.".to_owned(),
                    })
                }
            };
            match last_gates.get(&qubits) {
                Some(Some((name, _))) if name == op.hqslang() => {
                    last_gates.insert(qubits, None);
                }
                _ => {
                    for (key, _val) in last_gates.clone().iter() {
                        for qubit in [qubits.0, qubits.1].iter() {
                            if [key.0, key.1].contains(qubit) {
                                if let Some(Some((_name, operation))) = last_gates.get(key) {
                                    new_circuit.add_operation(operation.clone());
                                }
                                last_gates.insert(*key, None);
                            }
                        }
                    }
                    last_gates.insert(qubits, Some((op.hqslang().to_string(), op.clone())));
                }
            }
        } else {
            let qubits = match op.involved_qubits() {
                roqoqo::operations::InvolvedQubits::Set(qubits) => qubits.iter().cloned().collect(),
                _ => vec![],
            };
            for (key, _val) in last_gates.clone().iter() {
                for qubit in qubits.iter() {
                    if [key.0, key.1].contains(qubit) {
                        if let Some(Some((_name, operation))) = last_gates.get(key) {
                            new_circuit.add_operation(operation.clone());
                        }
                        last_gates.insert(*key, None);
                    }
                }
            }
            new_circuit.add_operation(op.clone())
        }
    }
    for (_name, operation) in last_gates.values().flatten() {
        new_circuit.add_operation(operation.clone());
    }
    if new_circuit.eq(&circuit) {
        Ok(new_circuit)
    } else {
        remove_identities(new_circuit)
    }
}
