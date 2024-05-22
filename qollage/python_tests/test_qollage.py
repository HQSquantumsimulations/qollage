# Copyright © 2023 HQS Quantum Simulations GmbH. All Rights Reserved.
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
# in compliance with the License. You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License
# is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
# or implied. See the License for the specific language governing permissions and limitations under
# the License.

import pytest  # type: ignore
import sys
from qollage import draw_circuit
from qoqo import Circuit, operations as ops  # type: ignore


def test_simple_draw() -> None:
    """Test drawing a circuit"""
    circuit = Circuit()
    circuit += ops.Hadamard(0)

    draw_circuit(circuit)


if __name__ == "__main__":
    pytest.main(sys.argv)
