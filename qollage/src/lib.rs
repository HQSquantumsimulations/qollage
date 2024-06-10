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

#![deny(missing_docs)]
#![warn(rustdoc::private_intra_doc_links)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::missing_doc_code_examples)]
#![warn(rustdoc::private_doc_tests)]
#![deny(missing_debug_implementations)]

//! # qollage
//!
//! Typst interface for qoqo.
//!
//! Translates circuits to Typst representation via the interface, and Create a Typst image.

use pyo3::prelude::*;

mod backend;
pub use backend::*;

#[pymodule]
fn qollage(_py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(draw_circuit, module)?)?;
    module.add_function(wrap_pyfunction!(save_circuit, module)?)?;
    module.add_function(wrap_pyfunction!(circuit_to_typst_str, module)?)?;
    module.add_function(wrap_pyfunction!(remove_two_qubit_gates_identities, module)?)?;
    Ok(())
}
