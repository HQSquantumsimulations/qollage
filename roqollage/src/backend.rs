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
use roqoqo::{operations::Operate, Circuit, RoqoqoBackendError};
use typst::{
    diag::{EcoString, FileError, FileResult, PackageError},
    eval::Tracer,
    foundations::{Bytes, Datetime},
    syntax::{FileId, Source},
    text::{Font, FontBook},
    visualize::Color,
    Library,
};

use crate::{add_gate, flatten_multiple_vec};

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
                RoqoqoBackendError::RoqoqoError(roqoqo::RoqoqoError::GenericError {
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
        .map_err(|_| RoqoqoBackendError::FileAlreadyExists {
            path: path.to_str().unwrap_or_default().to_owned(),
        })?;
        let url = "https://mirror.clientvps.com/CTAN/fonts/firamath/FiraMath-Regular.otf";

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
            std::fs::File::create(&path).map_err(|_| RoqoqoBackendError::FileAlreadyExists {
                path: path.to_str().unwrap_or_default().to_owned(),
            })?;
        std::fs::File::write(&mut file, &data).map_err(|_| {
            RoqoqoBackendError::FileAlreadyExists {
                path: path.to_str().unwrap_or_default().to_owned(),
            }
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
            _ => Err(RoqoqoBackendError::RoqoqoError(
                roqoqo::RoqoqoError::GenericError {
                    msg: format!(r#"Invalid initialization mode: {s}, use `state` or `qubit`."#),
                },
            )),
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
        let pattern = format!("replace_by_classical_len_{}", index);
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
        let pattern = format!("replace_by_n_qubits_plus_{}", boson);
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
            "all" => Ok(RenderPragmas::All),
            _ => Ok(RenderPragmas::Partial(
                s.split(',')
                    .filter(|&gate_name| gate_name.trim().starts_with("Pragma"))
                    .map(|gate_name| gate_name.trim().to_owned())
                    .collect(),
            )),
        }
    }
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
        match render_pragmas {
            RenderPragmas::All => (),
            RenderPragmas::None => {
                if operation.hqslang().starts_with("Pragma") {
                    continue;
                }
            }
            RenderPragmas::Partial(ref pragmas) => {
                if operation.hqslang().starts_with("Pragma")
                    && !pragmas.contains(&operation.hqslang().to_owned())
                {
                    continue;
                }
            }
        }
        add_gate(
            &mut circuit_gates,
            &mut bosonic_gates,
            &mut classical_gates,
            &mut circuit_lock,
            &mut bosonic_lock,
            &mut classical_lock,
            operation,
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
    let mut is_first = true;
    for (n_qubit, gates) in circuit_gates.iter().enumerate() {
        typst_str.push_str(&format!(
            "       lstick(${}${}), {}, 1, [\\ ],\n",
            match initializasion_mode {
                Some(InitializationMode::Qubit) => format!("q[{n_qubit}]"),
                Some(InitializationMode::State) | None => "|0>".to_owned(),
            },
            is_first.then(|| ", label: \"Qubits\"").unwrap_or_default(),
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
            is_first.then(|| ", label: \"Bosons\"").unwrap_or_default(),
            gates.join(", ")
        ));
        is_first = false;
    }
    for gates in classical_gates.iter() {
        typst_str.push_str(&format!("       {}, 1, [\\ ],\n", gates.join(", ")));
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
) -> Result<DynamicImage, RoqoqoBackendError> {
    let typst_str = circuit_into_typst_str(circuit, render_pragmas, initializasion_mode)?;
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
