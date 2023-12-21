/*********************** GNU General Public License 3.0 ***********************\
|                                                                              |
|  Copyright (C) 2023 Kevin Matthes                                            |
|                                                                              |
|  This program is free software: you can redistribute it and/or modify        |
|  it under the terms of the GNU General Public License as published by        |
|  the Free Software Foundation, either version 3 of the License, or           |
|  (at your option) any later version.                                         |
|                                                                              |
|  This program is distributed in the hope that it will be useful,             |
|  but WITHOUT ANY WARRANTY; without even the implied warranty of              |
|  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the               |
|  GNU General Public License for more details.                                |
|                                                                              |
|  You should have received a copy of the GNU General Public License           |
|  along with this program.  If not, see <https://www.gnu.org/licenses/>.      |
|                                                                              |
\******************************************************************************/

#![cfg(feature = "cff-create")]

use crate::{AppendAsLine, PatternWriter, ReadFile};
use std::{fmt::Display, path::PathBuf};
use sysexits::{ExitCode, Result};

struct Cff {
    abstrct: Option<String>,
    authors: Vec<CffAuthor>,
    cff_version: String,
    date_released: String,
    keywords: Vec<String>,
    license: Option<String>,
    message: String,
    repository_code: Option<String>,
    title: Option<String>,
    url: Option<String>,
    version: Option<String>,
}

impl Cff {
    fn new() -> Self {
        Self {
            abstrct: None,
            authors: Vec::new(),
            cff_version: "1.2.0".to_string(),
            date_released: chrono::Local::now()
                .date_naive()
                .format("%Y-%m-%d")
                .to_string(),
            keywords: Vec::new(),
            license: None,
            message: "Please cite this project using these information."
                .to_string(),
            repository_code: None,
            title: None,
            url: None,
            version: None,
        }
    }
}

impl Default for Cff {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Cff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();

        if let Some(abstrct) = &self.abstrct {
            s.append_as_line(format!("abstract: {abstrct}"));
        }

        if !self.authors.is_empty() {
            let mut authors = self.authors.clone();

            authors.sort();
            s.append_as_line("authors:");

            for author in authors {
                s.append_as_line(author.to_string());
            }
        }

        s.append_as_line(format!("cff-version: {}", self.cff_version));
        s.append_as_line(format!("date-released: {}", self.date_released));

        if !self.keywords.is_empty() {
            let mut keywords = self.keywords.clone();

            keywords.sort();
            s.append_as_line("keywords:");

            for keyword in keywords {
                s.append_as_line(format!("  - {keyword}"));
            }
        }

        if let Some(license) = &self.license {
            s.append_as_line(format!("license: {license}"));
        }

        s.append_as_line(format!("message: {}", self.message));

        if let Some(repository_code) = &self.repository_code {
            s.append_as_line(format!("repository-code: {repository_code}"));
        }

        if let Some(title) = &self.title {
            s.append_as_line(format!("title: {title}"));
        }

        if let Some(url) = &self.url {
            s.append_as_line(format!("url: {url}"));
        }

        if let Some(version) = &self.version {
            s.append_as_line(format!("version: {version}"));
        }

        write!(f, "{s}")
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CffAuthor {
    email: String,
    name: String,
}

impl CffAuthor {
    fn from_cargo_toml(author: &str) -> Result<Self> {
        if let Some((name, email)) = author.split_once('<') {
            Ok(Self {
                email: email
                    .trim()
                    .trim_matches(|c| "<>".contains(c))
                    .trim()
                    .to_string(),
                name: name.trim().to_string(),
            })
        } else {
            Err(ExitCode::DataErr)
        }
    }
}

impl Display for CffAuthor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "  - email: {}\n    name: {}", self.email, self.name)
    }
}

/// Extract the citation information from a given and valid CFF file.
#[derive(clap::Parser, Clone)]
#[command(visible_aliases = ["cffcreate", "mkcff"])]
pub struct Create {
    /// The manifest to read from.
    #[arg(long, short)]
    input_file: Option<PathBuf>,

    /// The type of manifest to analyse.
    #[arg(long, short, visible_aliases = ["mode"])]
    manifest_type: ManifestType,

    /// The output file to write to.
    #[arg(long, short)]
    output_file: Option<PathBuf>,
}

impl Create {
    /// Process the input data.
    ///
    /// # Errors
    ///
    /// See
    ///
    /// - [`ReadFile::read`]
    /// - [`sysexits::ExitCode::DataErr`]
    pub fn main(&self) -> Result<()> {
        self.wrap().main()
    }

    /// Create a new instance.
    #[must_use]
    pub const fn new(
        input_file: Option<PathBuf>,
        manifest_type: ManifestType,
        output_file: Option<PathBuf>,
    ) -> Self {
        Self {
            input_file,
            manifest_type,
            output_file,
        }
    }

    fn wrap(&self) -> Logic {
        Logic {
            cff: Cff::default(),
            cli: self.clone(),
        }
    }
}

macro_rules! manifest {
    ( Cargo.toml: $m:ident ! $( $f:literal -> $v:expr ),+ ) => {
        $(
            if $m.get($f).is_some() {
                $v = Some(
                    $m[$f]
                        .to_string()
                        .trim_matches('"')
                        .to_string()
                );
            }
        )+
    };
}

struct Logic {
    cff: Cff,
    cli: Create,
}

impl Logic {
    fn main(&mut self) -> Result<()> {
        self.read()?;
        self.cli
            .output_file
            .truncate(Box::new(self.cff.to_string()))
    }

    fn read(&mut self) -> Result<()> {
        match self.cli.manifest_type {
            ManifestType::Rust => self.rust(),
        }
    }

    fn rust(&mut self) -> Result<()> {
        if let Ok(manifest) = self.cli.input_file.read()?.parse::<toml::Table>()
        {
            let manifest = manifest.get("package").ok_or(ExitCode::DataErr)?;

            self.cff.keywords.push(self.cli.manifest_type.to_string());

            manifest!(Cargo.toml: manifest !
                "description" -> self.cff.abstrct,
                "license" -> self.cff.license,
                "repository" -> self.cff.repository_code,
                "name" -> self.cff.title,
                "homepage" -> self.cff.url,
                "version" -> self.cff.version
            );

            if manifest.get("categories").is_some() {
                for category in manifest["categories"]
                    .to_string()
                    .trim_matches(|c| "[]".contains(c))
                    .split(',')
                {
                    self.cff
                        .keywords
                        .push(category.trim().trim_matches('"').to_string());
                }
            }

            if manifest.get("keywords").is_some() {
                for keyword in manifest["keywords"]
                    .to_string()
                    .trim_matches(|c| "[]".contains(c))
                    .split(',')
                {
                    self.cff
                        .keywords
                        .push(keyword.trim().trim_matches('"').to_string());
                }
            }

            if manifest.get("authors").is_some() {
                for author in manifest["authors"]
                    .to_string()
                    .trim_matches(|c| "[]".contains(c))
                    .split(',')
                {
                    self.cff.authors.push(CffAuthor::from_cargo_toml(
                        author.trim().trim_matches('"'),
                    )?);
                }
            }

            Ok(())
        } else {
            Err(ExitCode::DataErr)
        }
    }
}

/// The type of manifest to analyse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestType {
    /// Analyse a given `Cargo.toml`.
    Rust,
}

crate::enum_trait!(ManifestType {
    Rust <- "rs"
});

crate::enum_trait!(ManifestType {
    Rust -> "Rust"
});

/******************************************************************************/
