# Copyright © 2023-2024 HQS Quantum Simulations GmbH. All Rights Reserved.
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

"""Qollage

qollage is the HQS python package to draw qoqo quantum circuits.

.. autosummary::
    :toctree: generated/

    draw_circuit
    save_circuit
    circuit_to_typst_str

"""

from qoqo import Circuit  # type: ignore
from typing import Optional

def draw_circuit(
    circuit: Circuit,
    pixel_per_point: float = 3.0,
    render_pragmas: str = "all",
    initialization_mode: Optional[str] = None,
    max_circuit_length: Optional[int] = None,
) -> None:
    """
    Displays the qoqo circuit as an image output

    ## Args:
         * circuit (Circuit): The qoqo circuit to draw.
         * pixel_per_point (float, optional): The pixels per point ration of the image.
            The higher the value, the bigger the image will be but the longer it will take to render.
         * render_pragmas (str, optional): How to render Pragmas operations:\n
             - `"all"` to render every pragmas.\n
             - `"none"` to not render any pragmas.\n
             - `"PragmaOperation1, PragmaOperation2"` to render only some pragmas.
         * initialization_mode (str, optional): What to display at the beginning of the circuit:\n
             - "state" for "|0>". Used if None. \n
             - "qubit" for "q[n]".\n
         * max_circuit_length (Optional(int)): The maximum number of gates per qubit before going to a new line.
             The default setting `None` does not create a new line.

    ## Raises:
         * TypeError: Circuit conversion error.
         * ValueError: Operation not supported | Memory limit exceeded if pixel_per_point is too large.
    """

def save_circuit(
    circuit: Circuit,
    path: Optional[str] = None,
    pixel_per_point: float = 3.0,
    render_pragmas: str = "all",
    initialization_mode: Optional[str] = None,
    max_circuit_length: Optional[int] = None,
) -> None:
    """
    Saves the qoqo circuit as a png image

    ## Args:
         * circuit (Circuit): The qoqo circuit to be saved.
         * path (str, optional): The path to where the image should be saved. "./circuit.png" will be used if None.
         * pixel_per_point (float, optional): The pixels per point ration of the image.
            The higher the value, the bigger the image will be but the longer it will take to render.
         * render_pragmas (str, optional): How to render Pragmas operations:\n
             - "all" to render every pragmas.\n
             - "none" to not render any pragmas.\n
             - "PragmaOperation1, PragmaOperation2" to render only some pragmas.
         * initialization_mode (str, optional): What to display at the beginning of the circuit:\n
             - "state" for "|0>". Used if None. \n
             - "qubit" for "q[n]".\n
         * max_circuit_length (Optional(int)): The maximum number of gates per qubit before going to a new line.
             The default setting `None` does not create a new line.

    ## Raises:
         * TypeError: Circuit conversion error
         * ValueError: Operation not supported. | Memory limit exceeded if pixel_per_point is too large. | Couldn't create the corresponding file.
    """

def circuit_to_typst_str(
    circuit: Circuit,
    render_pragmas: str = "all",
    initialization_mode: Optional[str] = None,
    max_circuit_length: Optional[int] = None,
) -> str:
    """
    Returns the circuit's representation in Typst.
    You can paste this string in https://typst.app/ and modify the circuit as you please.


    ## Args:
         * circuit (Circuit): The qoqo circuit to be saved.
         * render_pragmas (str, optional): How to render Pragmas operations:\n
             - "all" to render every pragmas.\n
             - "none" to not render any pragmas.\n
             - "PragmaOperation1, PragmaOperation2" to render only some pragmas.
         * initialization_mode (str, optional): What to display at the beginning of the circuit:\n
             - "state" for "|0>". Used if None. \n
             - "qubit" for "q[n]".\n
         * max_circuit_length (Optional(int)): The maximum number of gates per qubit before going to a new line.
             The default setting `None` does not create a new line.

    ## Returns:
         * str: The circuit's representation in Typst.

    ## Raises:
         * TypeError: Circuit conversion error
         * ValueError: Operation not supported. | Memory limit exceeded if pixel_per_point is too large. | Couldn't create the corresponding file.
    """
