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

use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Cursor, Write},
    path::PathBuf,
    str::FromStr,
};

use comemo::Prehashed;
use image::DynamicImage;
use roqoqo::{Circuit, RoqoqoBackendError, RoqoqoError};
use typst::{
    diag::{EcoString, FileError, FileResult, PackageError},
    eval::Tracer,
    foundations::{Bytes, Datetime},
    syntax::{FileId, Source},
    text::{Font, FontBook},
    visualize::Color,
    Library,
};

use crate::{add_gate, effective_len, flatten_multiple_vec};

/// Typst Backend
///
/// This backend can be used to process Typst input.
///
/// This backend will be used to compile a typst string to an image.
/// It has to implement the typst::World trait.
#[derive(Debug, Clone)]
pub struct TypstBackend {
    /// Typst standard library used by the backend.
    library: Prehashed<Library>,
    /// Metadata about a collection of fonts.
    book: Prehashed<FontBook>,
    /// Typst source file to be compiled.
    source: Source,
    /// Typst dependency files used during compilation.
    files: RefCell<HashMap<FileId, Bytes>>,
    /// Collection of fonts.
    fonts: Vec<Font>,
    /// Current time.
    time: time::OffsetDateTime,
    /// Path to the cache directory containing the font files and dependencies.
    dependencies: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// What to display at the left of the circuit.
pub enum InitializationMode {
    /// States |0>.
    State,
    /// Qubits q[n].
    Qubit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Choose how to render Pragmas operations.
pub enum RenderPragmas {
    /// Render no Pragmas operations.
    None,
    /// Render all Pragmas operations.
    All,
    /// Render Pragmas operations that listed.
    Partial(Vec<String>),
}

impl TypstBackend {
    /// Creates a new TypstBackend.
    ///
    /// # Arguments
    ///
    /// * `typst_str` - The typst source file.
    pub fn new(typst_str: String) -> Result<Self, RoqoqoBackendError> {
        let path = PathBuf::from(".qollage/fonts/FiraMath.otf");
        let bytes = match std::fs::read(path.clone()) {
            Ok(bytes) => bytes,
            Err(_) => {
                Self::download_font(path).map_err(|err| RoqoqoBackendError::NetworkError {
                    msg: format!("Couldn't download the font: {err}"),
                })?
            }
        };
        let buffer = Bytes::from(bytes);
        let fonts = Font::new(buffer.clone(), 0).map_or_else(std::vec::Vec::new, |font| vec![font]);
        Ok(Self {
            library: Prehashed::new(Library::default()),
            book: Prehashed::new(FontBook::from_fonts(&fonts)),
            source: Source::detached(typst_str.clone()),
            files: RefCell::new(HashMap::new()),
            fonts,
            time: time::OffsetDateTime::now_utc(),
            dependencies: PathBuf::from_str(".qollage/cache").map_err(|_| {
                RoqoqoBackendError::RoqoqoError(RoqoqoError::GenericError {
                    msg: "Couldn't access `.qollage/cache` directory".to_owned(),
                })
            })?,
        })
    }

    /// Downloads the FiraMath font.
    ///
    /// # Arguments
    ///
    /// * `path` `The path where to save the downloaded font file
    fn download_font(path: PathBuf) -> Result<Vec<u8>, RoqoqoBackendError> {
        std::fs::create_dir_all(
            path.parent()
                .unwrap_or(PathBuf::from(".qollage/fonts/").as_path()),
        )
        .map_err(|err| RoqoqoBackendError::GenericError {
            msg: format!("Couldn't create the font directory: {err}."),
        })?;
        let url = "https://mirrors.ctan.org/fonts/firamath/FiraMath-Regular.otf";

        let response = ureq::get(url)
            .call()
            .map_err(|err| RoqoqoBackendError::NetworkError {
                msg: format!("Couldn't download the font file: {err}."),
            })?;
        let mut data = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut data)
            .map_err(|err| RoqoqoBackendError::NetworkError {
                msg: format!("Couldn't read the font file: {err}."),
            })?;
        let mut file =
            std::fs::File::create(&path).map_err(|err| RoqoqoBackendError::GenericError {
                msg: format!("Couldn't create the font file: {err}."),
            })?;
        std::fs::File::write(&mut file, &data).map_err(|err| RoqoqoBackendError::GenericError {
            msg: format!("Couldn't write the font file: {err}."),
        })?;
        std::fs::read(path).map_err(|err| RoqoqoBackendError::GenericError {
            msg: format!("Couldn't read the font file: {err}"),
        })
    }

    /// Returns the typst dependency file.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the dependency file to load.
    fn load_file(&self, id: FileId) -> Result<Bytes, FileError> {
        if let Some(bytes) = self.files.borrow().get(&id) {
            return Ok(bytes.clone());
        }
        if let Some(package) = id.package() {
            let package_subdir =
                format!("{}/{}/{}", package.namespace, package.name, package.version);
            let package_path = self.dependencies.join(package_subdir);
            if !package_path.exists() {
                let url = format!(
                    "https://packages.typst.org/{}/{}-{}.tar.gz",
                    package.namespace, package.name, package.version,
                );
                let response = ureq::get(&url)
                    .call()
                    .map_err(|_| FileError::AccessDenied)?;
                let mut data = Vec::new();
                response
                    .into_reader()
                    .read_to_end(&mut data)
                    .map_err(|error| FileError::from_io(error, &package_path))?;
                let decompressed_data = zune_inflate::DeflateDecoder::new(&data)
                    .decode_gzip()
                    .map_err(|error| {
                        FileError::Package(PackageError::MalformedArchive(Some(
                            format!("Error during decompression:{error}.").into(),
                        )))
                    })?;
                let mut archive = tar::Archive::new(decompressed_data.as_slice());
                archive.unpack(&package_path).map_err(|error| {
                    FileError::Package(PackageError::MalformedArchive(Some(
                        format!("Error during unpacking:{error}.").into(),
                    )))
                })?;
            }
            if let Some(file_path) = id.vpath().resolve(&package_path) {
                let contents = std::fs::read(&file_path)
                    .map_err(|error| FileError::from_io(error, &file_path))?;
                self.files.borrow_mut().insert(id, contents.clone().into());
                return Ok(contents.into());
            }
        }
        Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
    }
}

impl typst::World for TypstBackend {
    /// The standard library.
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    /// Metadata about all known fonts.
    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    /// Access the main source file.
    fn main(&self) -> Source {
        self.source.clone()
    }

    /// Try to access the specified source file.
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            let bytes = self.file(id)?;
            let contents = std::str::from_utf8(&bytes).map_err(|_| FileError::InvalidUtf8)?;
            let contents = contents.trim_start_matches('\u{feff}');
            Ok(Source::new(id, contents.into()))
        }
    }

    /// Try to access the specified file.
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.load_file(id)
    }

    /// Try to access the font with the given index in the font book.
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    /// Get the current date.
    ///
    /// If no offset is specified, the local date should be chosen. Otherwise,
    /// the UTC date should be chosen with the corresponding offset in hours.
    ///
    /// If this function returns `None`, Typst's `datetime` function will
    /// return an error.
    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let offset =
            time::UtcOffset::from_hms(offset.unwrap_or_default().try_into().ok()?, 0, 0).ok()?;
        let time = self.time.checked_to_offset(offset)?;
        Some(Datetime::Date(time.date()))
    }
}

impl FromStr for InitializationMode {
    type Err = RoqoqoBackendError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "state" => Ok(InitializationMode::State),
            "qubit" => Ok(InitializationMode::Qubit),
            _ => Err(RoqoqoBackendError::RoqoqoError(RoqoqoError::GenericError {
                msg: format!(r#"Invalid initialization mode: {s}, use `state` or `qubit`."#),
            })),
        }
    }
}

/// Replaces `replace_by_classical_len_{n}` by n_qubits + n_bosons + n.
/// Needs to be done after going through all the circuit to know n_qubits and n_bosons.
///
/// # Arguments
///
/// * `bosonic_gate` - The bosonic gate in typst representation.
/// * `n_qubits` - The number of qubits.
/// * `n_bosons` - The number of bosons.
fn replace_classical_index(
    classical_gate: &String,
    n_qubits: usize,
    n_bosons: usize,
    n_classical: usize,
) -> String {
    let mut output = classical_gate.to_owned();
    for index in 0..n_classical + 1 {
        let pattern = format!("replace_by_classical_len_{index}");
        if output.contains(&pattern) {
            output = output.replace(&pattern, &(index + n_qubits + n_bosons).to_string());
        }
    }
    output
}

/// Replaces `replace_by_n_qubits_plus_{n}` by n_qubits + n.
/// Needs to be done after going through all the circuit to know n_qubits.
///
/// # Arguments
///
/// * `bosonic_gate` - The bosonic gate in typst representation.
/// * `n_qubits` - The number of qubits.
/// * `n_bosons` - The number of bosons.
fn replace_boson_index(bosonic_gate: &String, n_qubits: usize, n_bosons: usize) -> String {
    let mut output = bosonic_gate.to_owned();
    for boson in 0..n_bosons + 1 {
        let pattern = format!("replace_by_n_qubits_plus_{boson}");
        if output.contains(&pattern) {
            output = output.replace(&pattern, &(boson + n_qubits).to_string());
        }
    }
    output
}

impl FromStr for RenderPragmas {
    type Err = RoqoqoBackendError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(RenderPragmas::None),
            "all" | "" => Ok(RenderPragmas::All),
            _ => Ok(RenderPragmas::Partial(
                s.split(',')
                    .filter(|&gate_name| gate_name.trim().starts_with("Pragma"))
                    .map(|gate_name| gate_name.trim().to_owned())
                    .collect(),
            )),
        }
    }
}

/// Uses the Typst compiler to generate an image from the given typst string.
///
/// ## Arguments
///
/// * `typst_string` - The string to give to the typst compiler.
/// * `pixels_per_point` - The pixel per point ratio.
///
/// ## Returns
///
/// * `Ok(DynamicImage)` - The image generated from the typst string.
/// * `Err(RoqoqoBackendError)` - Error during the Typst compilation.
pub fn render_typst_str(
    typst_str: String,
    pixels_per_point: Option<f32>,
) -> Result<DynamicImage, RoqoqoBackendError> {
    let typst_backend = TypstBackend::new(typst_str)?;
    let mut tracer = Tracer::default();
    let doc = typst::compile(&typst_backend, &mut tracer).map_err(|err| {
        RoqoqoBackendError::GenericError {
            msg: format!(
                "Error during the Typst compilation: {}",
                err.iter()
                    .map(|source| format!(
                        "error: {}, Hints: {}",
                        source.message.as_str(),
                        source
                            .hints
                            .iter()
                            .map(EcoString::as_str)
                            .collect::<Vec<&str>>()
                            .join(","),
                    ))
                    .collect::<Vec<String>>()
                    .join("\n")
            ),
        }
    })?;
    let mut writer = Cursor::new(Vec::new());
    let background = Color::from_u8(255, 255, 255, 255);
    let pixmap = typst_render::render(
        &doc.pages
            .first()
            .ok_or("error")
            .map_err(|_| RoqoqoBackendError::GenericError {
                msg: "Typst document has no pages.".to_owned(),
            })?
            .frame,
        pixels_per_point.unwrap_or(3.0),
        background,
    );
    image::write_buffer_with_format(
        &mut writer,
        bytemuck::cast_slice(pixmap.pixels()),
        pixmap.width(),
        pixmap.height(),
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .map_err(|err| RoqoqoBackendError::GenericError {
        msg: err.to_string(),
    })?;
    let image = image::load_from_memory(&writer.into_inner()).map_err(|err| {
        RoqoqoBackendError::GenericError {
            msg: err.to_string(),
        }
    })?;
    Ok(image)
}

fn effective_split(vec: &mut Vec<String>, split_index: usize) -> (Vec<String>, Vec<String>, usize) {
    let mut first = vec![];
    let mut group_len = 0;
    while !vec.is_empty() && effective_len(first.as_slice()) < split_index.max(group_len) {
        let op = vec.remove(0);
        if op.contains("gategroup") {
            group_len = op
                .split(',')
                .nth(1)
                .unwrap_or("0")
                .trim()
                .parse::<usize>()
                .unwrap_or_default()
                + first.len();
        }
        first.push(op);
    }
    (first, vec.to_vec(), group_len)
}

fn split_gates(
    gates_vec: &mut [Vec<String>],
    max_len: usize,
    new_len_map: &HashMap<i64, usize>,
) -> Option<Vec<Vec<Vec<String>>>> {
    if !gates_vec.is_empty() && gates_vec[0].len() > max_len {
        let mut chunks: Vec<Vec<Vec<String>>> = vec![];
        let mut inner_chunks: Vec<Vec<String>> = vec![];
        for _ in 0..gates_vec.len() {
            inner_chunks.push(vec![]);
        }
        for _ in 0..(gates_vec[0].len() / max_len) {
            chunks.push(inner_chunks.clone());
        }
        for (ind, inner_vec) in gates_vec.iter_mut().enumerate() {
            let mut ind_chunk = 0_usize;
            let (first_part, mut remaining, _) = effective_split(
                inner_vec,
                max_len.max(
                    new_len_map
                        .get(&-1)
                        .map(usize::to_owned)
                        .unwrap_or_default(),
                ),
            );
            *inner_vec = first_part;
            while effective_len(remaining.as_slice()) > max_len {
                let (chunk, rest_of_remaining, _) = effective_split(
                    &mut remaining,
                    max_len.max(
                        new_len_map
                            .get(&(ind_chunk as i64))
                            .map(usize::to_owned)
                            .unwrap_or_default(),
                    ),
                );
                chunks[ind_chunk][ind].extend_from_slice(chunk.as_slice());
                ind_chunk += 1;
                remaining = rest_of_remaining;
            }
            if !remaining.is_empty() {
                chunks[ind_chunk][ind].extend_from_slice(remaining.as_slice());
            }
        }
        while chunks.last().is_some() && chunks.last().unwrap().iter().all(Vec::is_empty) {
            chunks.pop();
        }
        return Some(chunks);
    }
    None
}

fn split_in_chunk_preprocess(
    gates_vec: &[Vec<String>],
    max_len: usize,
    new_len_map: &mut HashMap<i64, usize>,
) -> bool {
    if !gates_vec.is_empty() && gates_vec[0].len() > max_len {
        let ref_map = new_len_map.clone();
        for inner_vec in gates_vec.iter() {
            let mut ind_chunk = 0_usize;
            let mut inner_vec = inner_vec.clone();
            let (_, mut remaining, chunk_group_len) = effective_split(
                &mut inner_vec,
                max_len.max(
                    new_len_map
                        .get(&-1)
                        .map(usize::to_owned)
                        .unwrap_or_default(),
                ),
            );
            if chunk_group_len != 0 {
                new_len_map.insert(
                    -1,
                    chunk_group_len.max(
                        new_len_map
                            .get(&(ind_chunk as i64))
                            .map(usize::to_owned)
                            .unwrap_or_default(),
                    ),
                );
            }
            while effective_len(remaining.as_slice()) > max_len {
                let (_, rest_of_remaining, chunk_group_len) = effective_split(
                    &mut remaining,
                    max_len.max(
                        new_len_map
                            .get(&(ind_chunk as i64))
                            .map(usize::to_owned)
                            .unwrap_or_default(),
                    ),
                );
                if chunk_group_len != 0 {
                    new_len_map.insert(
                        ind_chunk as i64,
                        chunk_group_len.max(
                            new_len_map
                                .get(&(ind_chunk as i64))
                                .map(usize::to_owned)
                                .unwrap_or_default(),
                        ),
                    );
                }
                ind_chunk += 1;
                remaining = rest_of_remaining;
            }
        }
        return ref_map.eq(new_len_map);
    }
    false
}

/// Converts a qoqo circuit to a typst string.
///
///  ## Arguments
///
/// * `circuit` - The circuit to convert.
/// * `render_pragmas` - Whether to render Pragma Operations or not.
/// * `initialization_mode` - The initialization mode of the circuit representation.
///
/// ## Returns
///
/// * `String` - The string representation of the circuit in Typst.
pub fn circuit_into_typst_str(
    circuit: &Circuit,
    render_pragmas: RenderPragmas,
    initializasion_mode: Option<InitializationMode>,
    max_length: Option<usize>,
) -> Result<String, RoqoqoBackendError> {
    let mut typst_str = r#"#set page(width: auto, height: auto, margin: 5pt)
#show math.equation: set text(font: "Fira Math")
#{ 
    import "@preview/quill:0.2.1": *
    quantum-circuit(
"#
    .to_owned();
    let mut circuit_gates: Vec<Vec<String>> = Vec::new();
    let mut bosonic_gates: Vec<Vec<String>> = Vec::new();
    let mut classical_gates: Vec<Vec<String>> = Vec::new();
    let mut circuit_lock: Vec<(usize, usize)> = Vec::new();
    let mut bosonic_lock: Vec<(usize, usize)> = Vec::new();
    let mut classical_lock: Vec<(usize, usize)> = Vec::new();
    for operation in circuit.iter() {
        add_gate(
            &mut circuit_gates,
            &mut bosonic_gates,
            &mut classical_gates,
            &mut circuit_lock,
            &mut bosonic_lock,
            &mut classical_lock,
            operation,
            &render_pragmas,
        )?;
    }
    let n_qubits = circuit_gates.len();
    let n_bosons = bosonic_gates.len();
    let n_classical = classical_gates.len();
    flatten_multiple_vec(
        &mut circuit_gates,
        &mut bosonic_gates,
        (0..n_qubits).collect::<Vec<usize>>().as_slice(),
        (0..n_bosons).collect::<Vec<usize>>().as_slice(),
    );
    flatten_multiple_vec(
        &mut circuit_gates,
        &mut classical_gates,
        (0..n_qubits).collect::<Vec<usize>>().as_slice(),
        (0..n_classical).collect::<Vec<usize>>().as_slice(),
    );
    flatten_multiple_vec(
        &mut bosonic_gates,
        &mut classical_gates,
        (0..n_bosons).collect::<Vec<usize>>().as_slice(),
        (0..n_classical).collect::<Vec<usize>>().as_slice(),
    );
    let mut additional_circuit_gates = None;
    let mut additional_bosonic_gates = None;
    let mut additional_classical_gates = None;
    if let Some(max_circuit_length) = max_length {
        let mut new_len_map: HashMap<i64, usize> = HashMap::new();
        while !split_in_chunk_preprocess(&circuit_gates, max_circuit_length, &mut new_len_map) {}
        additional_circuit_gates =
            split_gates(&mut circuit_gates, max_circuit_length, &new_len_map);
        additional_bosonic_gates =
            split_gates(&mut bosonic_gates, max_circuit_length, &new_len_map);
        additional_classical_gates =
            split_gates(&mut classical_gates, max_circuit_length, &new_len_map);
    }
    let mut is_first = true;
    for (n_qubit, gates) in circuit_gates.iter().enumerate() {
        typst_str.push_str(&format!(
            "       lstick(${}${}), {}, 1, [\\ ],\n",
            match initializasion_mode {
                Some(InitializationMode::Qubit) => format!("q[{n_qubit}]"),
                Some(InitializationMode::State) | None => "|0>".to_owned(),
            },
            if is_first {
                ", label: \"Qubits\""
            } else {
                Default::default()
            },
            gates
                .iter()
                .map(|gate| {
                    if gate.contains("replace_by_n_qubits_") {
                        replace_boson_index(gate, n_qubits, n_bosons)
                    } else if gate.contains("replace_by_classical_len_") {
                        replace_classical_index(gate, n_qubits, n_bosons, n_classical)
                    } else {
                        gate.to_owned()
                    }
                })
                .collect::<Vec<String>>()
                .join(", ")
        ));
        is_first = false;
    }
    is_first = true;
    for (n_boson, gates) in bosonic_gates.iter().enumerate() {
        typst_str.push_str(&format!(
            "       lstick(${}${}), {}, 1, [\\ ],\n",
            match initializasion_mode {
                Some(InitializationMode::Qubit) => format!("q[{n_boson}]"),
                Some(InitializationMode::State) | None => "|0>".to_owned(),
            },
            if is_first {
                ", label: \"Bosons\""
            } else {
                Default::default()
            },
            gates.join(", ")
        ));
        is_first = false;
    }
    for gates in classical_gates.iter() {
        typst_str.push_str(&format!("       {}, 1, [\\ ],\n", gates.join(", ")));
    }
    if max_length.is_some()
        && (additional_circuit_gates.is_some()
            || additional_bosonic_gates.is_some()
            || additional_classical_gates.is_some())
    {
        let number_of_chunks = additional_circuit_gates
            .as_ref()
            .map(|v| v.len())
            .unwrap_or({
                additional_bosonic_gates
                    .as_ref()
                    .map(|v| v.len())
                    .unwrap_or(
                        additional_classical_gates
                            .as_ref()
                            .map(|v| v.len())
                            .unwrap_or_default(),
                    )
            });
        for chunk_number in 0..number_of_chunks {
            if let Some(ref add_circuit_gates) = additional_circuit_gates {
                let current_chunk = &add_circuit_gates[chunk_number];
                is_first = true;
                for gates in current_chunk.iter() {
                    typst_str.push_str(&format!(
                        "{}       lstick($${}), {}, 1, [\\ ],\n",
                        if is_first {
                            "[\\ ],\n"
                        } else {
                            Default::default()
                        },
                        if is_first {
                            ", label: \"Qubits\""
                        } else {
                            Default::default()
                        },
                        gates
                            .iter()
                            .map(|gate| {
                                if gate.contains("replace_by_n_qubits_") {
                                    replace_boson_index(gate, n_qubits, n_bosons)
                                } else if gate.contains("replace_by_classical_len_") {
                                    replace_classical_index(gate, n_qubits, n_bosons, n_classical)
                                } else {
                                    gate.to_owned()
                                }
                            })
                            .collect::<Vec<String>>()
                            .join(", ")
                    ));
                    is_first = false;
                }
            }
            if let Some(ref add_bosonic_gates) = additional_bosonic_gates {
                let current_chunk = &add_bosonic_gates[chunk_number];
                is_first = true;
                for gates in current_chunk.iter() {
                    typst_str.push_str(&format!(
                        "{}       lstick($${}), {}, 1, [\\ ],\n",
                        if is_first {
                            "[\\ ],\n"
                        } else {
                            Default::default()
                        },
                        if is_first {
                            ", label: \"Bosons\""
                        } else {
                            Default::default()
                        },
                        gates.join(", ")
                    ));
                    is_first = false;
                }
            }
            if let Some(ref add_classical_gates) = additional_classical_gates {
                let current_chunk = &add_classical_gates[chunk_number];
                for (index, gates) in current_chunk.clone().iter_mut().enumerate() {
                    gates.insert(0, classical_gates[index][1].clone());
                    typst_str.push_str(&format!(
                        "{}       lstick($$), {}, 1, [\\ ],\n",
                        if is_first {
                            "[\\ ],\n"
                        } else {
                            Default::default()
                        },
                        gates.join(", "),
                    ));
                }
            }
        }
    }
    typst_str = typst_str
        .strip_suffix(" [\\ ],\n")
        .map(str::to_owned)
        .unwrap_or(typst_str);
    typst_str.push_str(")\n}\n");
    Ok(typst_str)
}

/// Converts a qoqo circuit to an image.
///
///  ## Arguments
///
/// * `circuit` - The circuit to convert.
/// * `pixels_per_point` - The pixel per point ratio.
/// * `render_pragmas` - Whether to render Pragma Operations or not.
/// * `initialization_mode` - The initialization mode of the circuit representation.
///
/// ## Returns
///
/// * DynamicImage: The image reprensenting the circuit.
pub fn circuit_to_image(
    circuit: &Circuit,
    pixels_per_point: Option<f32>,
    render_pragmas: RenderPragmas,
    initializasion_mode: Option<InitializationMode>,
    max_length: Option<usize>,
) -> Result<DynamicImage, RoqoqoBackendError> {
    let typst_str =
        circuit_into_typst_str(circuit, render_pragmas, initializasion_mode, max_length)?;
    render_typst_str(typst_str, pixels_per_point)
}
