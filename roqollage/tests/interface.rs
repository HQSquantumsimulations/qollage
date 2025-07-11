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
//! Testing the roqollage Interface

use ndarray::array;
use num_complex::Complex64;
use qoqo_calculator::CalculatorFloat;
use roqollage::add_gate;
use roqoqo::{operations::*, Circuit};
use std::collections::HashMap;
use std::f64::consts::PI;
use test_case::test_case;

// / Test that all operations return the correct gate declaration
#[test_case(Operation::from(PauliX::new(0)); "PauliX")]
#[test_case(Operation::from(PauliY::new(0)); "PauliY")]
#[test_case(Operation::from(PauliZ::new(0)); "PauliZ")]
#[test_case(Operation::from(Hadamard::new(0)); "Hadamard")]
#[test_case(Operation::from(SGate::new(0)); "SGate")]
#[test_case(Operation::from(TGate::new(0)); "TGate")]
#[test_case(Operation::from(RotateX::new(0, CalculatorFloat::from(-PI))); "RotateX")]
#[test_case(Operation::from(RotateY::new(0, CalculatorFloat::from(-PI))); "RotateY")]
#[test_case(Operation::from(RotateZ::new(0, CalculatorFloat::from(-PI))); "RotateZ")]
#[test_case(Operation::from(CNOT::new(0, 1)); "CNOT")]
#[test_case(Operation::from(ControlledPauliZ::new(0, 1)); "ControlledPauliZ")]
#[test_case(Operation::from(SWAP::new(0, 1)); "SWAP")]
#[test_case(Operation::from(ISwap::new(0, 1)); "ISwap")]
#[test_case(Operation::from(SqrtISwap::new(0, 1)); "SqrtISwap")]
#[test_case(Operation::from(InvSqrtISwap::new(0, 1)); "InvSqrtISwap")]
#[test_case(Operation::from(FSwap::new(0, 1)); "FSwap")]
#[test_case(Operation::from(MeasureQubit::new(0,"ro".to_owned(), 0)); "MeasureQubit")]
#[test_case(Operation::from(Toffoli::new(0, 1, 2)); "Toffoli")]
#[test_case(Operation::from(PragmaConditional::new("q".to_owned(), 0, Circuit::new())); "PragmaConditionalEmpty")]
#[test_case(Operation::from(PragmaConditional::new("q".to_owned(), 0, [Operation::from(RotateX::new(0,CalculatorFloat::from("theta"))), Operation::from(RotateX::new(1,CalculatorFloat::from("pi")))].into_iter().collect())); "PragmaConditional")]
#[test_case(Operation::from(PragmaLoop::new(CalculatorFloat::Float(5.2), vec![Operation::from(RotateX::new(0, CalculatorFloat::from("theta"))), Operation::from(RotateX::new(1, CalculatorFloat::PI))].into_iter().collect())); "PragmaLoop")]
#[test_case(Operation::from(MultiQubitZZ::new(vec![0, 1], CalculatorFloat::from(-PI))); "MultiqubitZZ")]
#[test_case(Operation::from(XY::new(0, 1, CalculatorFloat::PI)); "XY")]
#[test_case(Operation::from(InvSqrtPauliX::new(0)); "InvSqrtPauliX")]
#[test_case(Operation::from(SqrtPauliX::new(0)); "SqrtPauliX")]
#[test_case(Operation::from(Identity::new(0)); "Identity")]
#[test_case(Operation::from(PMInteraction::new(0, 1, CalculatorFloat::from(0.069))); "PMInteraction")]
#[test_case(Operation::from(GivensRotation::new(0, 1, CalculatorFloat::from("5"), CalculatorFloat::FRAC_1_SQRT_2)); "GivensRotation")]
#[test_case(Operation::from(GivensRotationLittleEndian::new(0, 1, CalculatorFloat::from("5"), CalculatorFloat::FRAC_1_SQRT_2)); "GivensRotationLittleEndian")]
#[test_case(Operation::from(PhaseShiftedControlledZ::new(0, 1, CalculatorFloat::FRAC_PI_4)); "PhaseShiftedControlledZ")]
#[test_case(Operation::from(PhaseShiftedControlledPhase::new(0, 1, CalculatorFloat::PI, CalculatorFloat::FRAC_PI_4)); "PhaseShiftedControlledPhase")]
#[test_case(Operation::from(PhaseShiftState1::new(4, CalculatorFloat::FRAC_PI_4)); "PhaseShiftState1")]
#[test_case(Operation::from(MolmerSorensenXX::new(0, 1)); "MolmerSorensenXX")]
#[test_case(Operation::from(VariableMSXX::new(0, 1, CalculatorFloat::FRAC_PI_4)); "VariableMSXX")]
#[test_case(Operation::from(ControlledPauliY::new(0, 1)); "ControlledPauliY")]
#[test_case(Operation::from(ControlledPhaseShift::new(0, 1, CalculatorFloat::FRAC_PI_2)); "ControlledPhaseShift")]
#[test_case(Operation::from(RotateXY::new(0, CalculatorFloat::from("1"), CalculatorFloat::FRAC_PI_2)); "RotateXY")]
#[test_case(Operation::from(ControlledControlledPauliZ::new(0, 1, 2)); "ControlledControlledPauliZ")]
#[test_case(Operation::from(ControlledControlledPhaseShift::new(0, 1, 2, CalculatorFloat::PI)); "ControlledControlledPhaseShift")]
#[test_case(Operation::from(ComplexPMInteraction::new(1,0, 1.0.into(), 2.0.into())); "ComplexPMInteraction")]
#[test_case(Operation::from(Qsim::new(1,0, 0.5.into(), 1.0.into(), 0.5.into())); "Qsim")]
#[test_case(Operation::from(Fsim::new(1,0, 0.5.into(), 1.0.into(), 0.5.into())); "Fsim")]
#[test_case(Operation::from(SpinInteraction::new(1,0, 1.0.into(), 2.0.into(), 3.0.into())); "SpinInteraction")]
#[test_case(Operation::from(Bogoliubov::new(1,0, 1.0.into(), 2.0.into())); "Bogoliubov")]
#[test_case(Operation::from(PhaseShiftedControlledZ::new(1,0, 3.0.into())); "PhaseShifterControlledZ")]
#[test_case(Operation::from(ControlledRotateX::new(0, 1, 0.1.into())); "ControlledRotateX")]
#[test_case(Operation::from(ControlledRotateXY::new(0, 1, 0.1.into(), 0.2.into())); "ControlledRotateXY")]
#[test_case(Operation::from(EchoCrossResonance::new(0, 1)); "EchoCrossResonance")]
#[test_case(Operation::from(RotateAroundSphericalAxis::new(0, 1.0.into(), 0.5.into(), 1.0.into())); "RotateAroundSphericalAxis")]
#[test_case(Operation::from(SingleQubitGate::new(0,0.5.into(),  0.5.into(), 0.5.into(), 0.5.into(), 0.5.into()));"SingleQubitGate")]
#[test_case(Operation::from(GPi::new(0, 0.1.into()));"GPi")]
#[test_case(Operation::from(GPi2::new(0, 0.1.into()));"GPi2")]
#[test_case(Operation::from(MultiQubitMS::new(vec![0,1,2,3], 1.0.into())); "MultiQubitMS")]
#[test_case(Operation::from(Squeezing::new(0, 1.0.into(), 0.0.into())); "Squeezing")]
#[test_case(Operation::from(PhaseShift::new(0, 1.0.into())); "PhaseShift")]
#[test_case(Operation::from(PhaseDisplacement::new(0, 1.0.into(), 0.1.into())); "PhaseDisplacement")]
#[test_case(Operation::from(QuantumRabi::new(1, 0, 1.0.into()));"QuantumRabi")]
#[test_case(Operation::from(LongitudinalCoupling::new(1, 2, 1.0.into()));"LongitudinalCoupling")]
#[test_case(Operation::from(JaynesCummings::new(1, 2, 1.0.into()));"JaynesCummings")]
#[test_case(Operation::from(SingleExcitationLoad::new(1, 1));"SingleExcitationLoad")]
#[test_case(Operation::from(SingleExcitationStore::new(1, 1));"SingleExcitationStore")]
#[test_case(Operation::from(CZQubitResonator::new(1, 2));"CZQubitResonator")]
#[test_case(Operation::from(PragmaSetNumberOfMeasurements::new(3, "ro".into())); "PragmaSetNumberOfMeasurements")]
#[test_case(Operation::from(PragmaRepeatGate::new(3)); "PragmaRepeatGate")]
#[test_case(Operation::from(PragmaGeneralNoise::new(0, 1.0.into(),  array![[0.1, 0.0, 0.0],[0.0, 0.0, 0.0],[0.0, 0.0, 0.0]])); "PragmaGeneralNoise")]
#[test_case(Operation::from(PragmaBoostNoise::new(0.5.into())); "PragmaBoostNoise")]
#[test_case(Operation::from(PragmaStopParallelBlock::new(vec![0, 1], 0.5.into())); "PragmaStopParallelBlock")]
#[test_case(Operation::from(PragmaGlobalPhase::new(0.5.into())); "PragmaGlobalPhase")]
#[test_case(Operation::from(PragmaStartDecompositionBlock::new(vec![0, 1], HashMap::new())); "PragmaStartDecompositionBlock")]
#[test_case(Operation::from(PragmaStopDecompositionBlock::new(vec![0, 1])); "PragmaStopDecompositionBlock")]
#[test_case(Operation::from(DefinitionUsize::new("ro".into(), 2, false)); "DefinitionUsize")]
#[test_case(Operation::from(InputSymbolic::new("ro".into(), 2.0)); "InputSymbolic")]
#[test_case(Operation::from(PragmaDamping::new(0, 0.01.into(),  2.0.into())); "PragmaDamping001")]
#[test_case(Operation::from(PragmaDephasing::new(0, 0.01.into(),  2.0.into())); "PragmaDephasing")]
#[test_case(Operation::from(PragmaGetPauliProduct::new(HashMap::from([(0, 0), (1,1), (2,2), (3,3)]), "ro".into(), roqoqo::Circuit::new(),)); "PragmaGetPauliProduct")]
#[test_case(Operation::from(PragmaActiveReset::new(0)); "PragmaActiveReset")]
#[test_case(Operation::from(PragmaSleep::new(vec![0],0.0.into())); "PragmaSleep")]
#[test_case(Operation::from(PragmaRepeatedMeasurement::new( "ro".to_string(), 10, None)); "PragmaRepeatedMeasurement")]
#[test_case(Operation::from(DefinitionBit::new("ro".into(), 2, false)); "DefinitionBit")]
#[test_case(Operation::from(DefinitionFloat::new("ro".into(), 2, false)); "DefinitionFloat")]
#[test_case(Operation::from(DefinitionComplex::new("ro".into(), 2, false)); "DefinitionComplex")]
#[test_case(Operation::from(PragmaGetOccupationProbability::new("ro".into(), None)); "PragmaGetOccupationProbability")]
#[test_case(Operation::from(InputBit::new(String::from("test"), 1, false)); "InputBit")]
#[test_case(Operation::from(PhaseShiftState0::new(0, CalculatorFloat::PI)); "PhaseShiftState0")]
#[test_case(Operation::from(PhotonDetection::new(0, "ro".to_owned(), 0)); "PhotonDetection")]
#[test_case(Operation::from(PragmaGetDensityMatrix::new("ro".to_owned(), None)); "PragmaGetDensityMatrix")]
#[test_case(Operation::from(PragmaChangeDevice::new(&Operation::from(PragmaOverrotation::new("RotateX".to_owned(), vec![0,1], 0.1, 0.1))).unwrap()); "PragmaChangeDevice")]
#[test_case(Operation::from(PragmaRandomNoise::new(0, CalculatorFloat::ONE, CalculatorFloat::from(0.1), CalculatorFloat::from(0.1))); "PragmaRandomNoise")]
#[test_case(Operation::from(PragmaDepolarising::new(0, CalculatorFloat::from("1.0"), CalculatorFloat::from("theta"))); "PragmaDepolarising")]
#[test_case(Operation::from(PragmaOverrotation::new("RotateX".to_owned(), vec![0,1], 0.1, 0.1)); "PragmaOverrotation")]
#[test_case(Operation::from(PragmaControlledCircuit::new(10, vec![Operation::from(RotateX::new(0, CalculatorFloat::from("theta"))), Operation::from(RotateX::new(1, CalculatorFloat::PI))].into_iter().collect())); "PragmaControlledCircuit")]
#[test_case(Operation::from(PragmaAnnotatedOp::new(PauliX::new(0).into(), "test".to_string())); "PragmaAnnotatedOp")]
#[test_case(Operation::from(PragmaGetStateVector::new("ro".to_owned(), None)); "PragmaGetStateVector")]
#[test_case(Operation::from(PragmaSetStateVector::new(array![Complex64::new(0.0, 0.0),Complex64::new(1.0 / (2.0_f64).sqrt(), 0.0),Complex64::new(-1.0 / (2.0_f64).sqrt(), 0.0),Complex64::new(0.0, 0.0) ])); "PragmaSetStateVector")]
#[test_case(Operation::from(PragmaSetDensityMatrix::new(array![[Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],[Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)],])); "PragmaSetDensityMatrix")]
#[test_case(Operation::from(InvSqrtPauliY::new(0)); "InvSqrtPauliY")]
#[test_case(Operation::from(SqrtPauliY::new(0)); "SqrtPauliY")]
#[test_case(Operation::from(InvSGate::new(0)); "InvSGate")]
#[test_case(Operation::from(InvTGate::new(0)); "InvTGate")]
#[test_case(Operation::from(SXGate::new(0)); "SXGate")]
#[test_case(Operation::from(InvSXGate::new(0)); "InvSXGate")]
#[test_case(Operation::from(ControlledSWAP::new(0, 1, 2)); "ControlledSWAP")]
#[test_case(Operation::from(TripleControlledPauliX::new(0, 1, 2, 3)); "TripleControlledPauliX")]
#[test_case(Operation::from(TripleControlledPauliZ::new(0, 1, 2, 3)); "TripleControlledPauliZ")]
#[test_case(Operation::from(PhaseShiftedControlledControlledZ::new(0, 1, 2, CalculatorFloat::FRAC_PI_4)); "PhaseShiftedControlledControlledZ")]
#[test_case(Operation::from(PhaseShiftedControlledControlledPhase::new(0, 1, 2, CalculatorFloat::FRAC_PI_4, CalculatorFloat::ZERO)); "PhaseShiftedControlledControlledPhase")]
#[test_case(Operation::from(TripleControlledPhaseShift::new(0, 1, 2, 3, CalculatorFloat::ZERO)); "TripleControlledPhaseShift")]
#[test_case(Operation::from(MultiQubitCNOT::new([0, 1, 2, 3].to_vec())); "MultiQubitCNOT")]
#[test_case(Operation::from(QFT::new([0, 1, 2, 3].to_vec(), false, false)); "QFT")]
fn test_add_gate(operation: Operation) {
    let mut circuit_gates: Vec<Vec<String>> = Vec::new();
    let mut bosonic_gates: Vec<Vec<String>> = Vec::new();
    let mut classical_gates: Vec<Vec<String>> = Vec::new();
    let mut circuit_lock: Vec<(usize, usize)> = Vec::new();
    let mut bosonic_lock: Vec<(usize, usize)> = Vec::new();
    let mut classical_lock: Vec<(usize, usize)> = Vec::new();
    assert!(add_gate(
        &mut circuit_gates,
        &mut bosonic_gates,
        &mut classical_gates,
        &mut circuit_lock,
        &mut bosonic_lock,
        &mut classical_lock,
        &operation,
        &roqollage::RenderPragmas::All,
    )
    .is_ok());
}

#[test_case(Operation::from(PragmaStartDecompositionBlock::new(vec![], HashMap::new())); "PragmaStartDecompositionBlock")]
#[test_case(Operation::from(PragmaStopDecompositionBlock::new(vec![])); "PragmaStopDecompositionBlock")]
#[test_case(Operation::from(PragmaOverrotation::new("RotateX".to_owned(), vec![], 0.0001, 9990.1)); "PragmaOverrotation")]
#[test_case(Operation::from(PragmaStopParallelBlock::new(vec![], 0.5.into())); "PragmaStopParallelBlock")]
#[test_case(Operation::from(PragmaSleep::new(vec![], 0.0012.into())); "PragmaSleep")]
#[test_case(Operation::from(MultiQubitMS::new(vec![], 0.000321.into())); "MultiQubitMS")]
#[test_case(Operation::from(MultiQubitZZ::new(vec![], 0.0.into())); "MultiQubitZZ")]
#[test_case(Operation::from(PragmaGetDensityMatrix::new("ro".to_owned(), Some(Circuit::new()))); "PragmaGetDensityMatrix")]
#[test_case(Operation::from(PragmaGetStateVector::new("ro".to_owned(), Some(Circuit::new()))); "PragmaGetStateVector")]
#[test_case(Operation::from(PragmaRepeatedMeasurement::new("ro".to_owned(), 5, Some(HashMap::from([])))); "PragmaRepeatedMeasurement")]
#[test_case(Operation::from(PragmaGetOccupationProbability::new("ro".to_owned(), Some(Circuit::new()))); "PragmaGetOccupationProbability")]
#[test_case(Operation::from(PragmaGetPauliProduct::new(HashMap::from([(4, 4)]), "ro".into(), Circuit::new(),)); "PragmaGetPauliProduct")]
#[test_case(Operation::from(PragmaGetPauliProduct::new(HashMap::from([]), "ro".into(), Circuit::new(),)); "PragmaGetPauliProduct2")]
#[test_case(Operation::from(PragmaAnnotatedOp::new(InputBit::new("ro".to_owned(), 0, true).into(), "test".to_string())); "PragmaAnnotatedOp")]
#[test_case(Operation::from(PragmaControlledCircuit::new(10, vec![Operation::from(InputBit::new("ro".to_owned(), 0, true))].into_iter().collect())); "PragmaControlledCircuit")]
#[test_case(Operation::from(MultiQubitCNOT::new([].to_vec())); "MultiQubitCNOT")]
#[test_case(Operation::from(QFT::new([].to_vec(), false, false)); "QFT")]
fn test_add_gate_errors(operation: Operation) {
    let mut circuit_gates: Vec<Vec<String>> = Vec::new();
    let mut bosonic_gates: Vec<Vec<String>> = Vec::new();
    let mut classical_gates: Vec<Vec<String>> = Vec::new();
    let mut circuit_lock: Vec<(usize, usize)> = Vec::new();
    let mut bosonic_lock: Vec<(usize, usize)> = Vec::new();
    let mut classical_lock: Vec<(usize, usize)> = Vec::new();
    assert!(add_gate(
        &mut circuit_gates,
        &mut bosonic_gates,
        &mut classical_gates,
        &mut circuit_lock,
        &mut bosonic_lock,
        &mut classical_lock,
        &operation,
        &roqollage::RenderPragmas::All
    )
    .is_err(),);
}

#[cfg(feature = "unstable_operation_definition")]
#[test_case(Operation::from(GateDefinition::new(vec![Operation::from(RotateX::new(0, CalculatorFloat::from("theta"))), Operation::from(RotateX::new(1, CalculatorFloat::PI))].into_iter().collect(), "test_gate".to_owned(), vec![0, 1], vec!["theta".to_owned()])); "GateDefinition")]
#[test_case(Operation::from(CallDefinedGate::new("test".to_owned(), vec![0, 1], vec![CalculatorFloat::from("3.14")])); "CallDefinedGate")]
fn test_add_gate_unstable(operation: Operation) {
    let mut circuit_gates: Vec<Vec<String>> = Vec::new();
    let mut bosonic_gates: Vec<Vec<String>> = Vec::new();
    let mut classical_gates: Vec<Vec<String>> = Vec::new();
    let mut circuit_lock: Vec<(usize, usize)> = Vec::new();
    let mut bosonic_lock: Vec<(usize, usize)> = Vec::new();
    let mut classical_lock: Vec<(usize, usize)> = Vec::new();
    assert!(add_gate(
        &mut circuit_gates,
        &mut bosonic_gates,
        &mut classical_gates,
        &mut circuit_lock,
        &mut bosonic_lock,
        &mut classical_lock,
        &operation,
        &roqollage::RenderPragmas::All,
    )
    .is_ok());
}

#[cfg(feature = "unstable_operation_definition")]
#[test_case(Operation::from(GateDefinition::new(vec![Operation::from(InputBit::new("ro".to_owned(), 0, true))].into_iter().collect(), "test_gate".to_owned(), vec![0, 1], vec!["theta".to_owned()])); "GateDefinition")]
#[test_case(Operation::from(CallDefinedGate::new("test".to_owned(), vec![], vec![CalculatorFloat::from("3.14")])); "CallDefinedGate")]
fn test_add_gate_unstable_errors(operation: Operation) {
    let mut circuit_gates: Vec<Vec<String>> = Vec::new();
    let mut bosonic_gates: Vec<Vec<String>> = Vec::new();
    let mut classical_gates: Vec<Vec<String>> = Vec::new();
    let mut circuit_lock: Vec<(usize, usize)> = Vec::new();
    let mut bosonic_lock: Vec<(usize, usize)> = Vec::new();
    let mut classical_lock: Vec<(usize, usize)> = Vec::new();
    assert!(add_gate(
        &mut circuit_gates,
        &mut bosonic_gates,
        &mut classical_gates,
        &mut circuit_lock,
        &mut bosonic_lock,
        &mut classical_lock,
        &operation,
        &roqollage::RenderPragmas::All
    )
    .is_err(),);
}
