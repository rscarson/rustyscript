use maybe_path::MaybePathBuf;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};

/// Creates a static module
///
/// This is just a macro around [`Module::new_static`]
///
/// # Arguments
/// * `filename` - A string representing the filename of the module.
/// * `contents` - A string containing the contents of the module.
///
/// Note that the contents argument is optional;
/// if not provided, the macro will attempt to include the file at the given path.
///
/// # Example
///
/// ```rust
/// use rustyscript::{ module, Module };
///
/// const MY_SCRIPT: Module = module!(
///     "filename.js",
///     "export const myValue = 42;"
/// );
/// ```
#[macro_export]
macro_rules! module {
    ($filename:literal, $contents:literal) => {
        $crate::Module::new_static($filename, $contents)
    };

    ($filename:literal) => {
        Module::new_static($filename, include_str!($filename))
    };
}

/// Creates a static module based on a statically included file
///
/// # Arguments
/// * `filename` - A string representing the filename of the module.
///
/// See [module] for an example
#[macro_export]
macro_rules! include_module {
    ($filename:literal) => {
        Module::new_static($filename, include_str!($filename))
    };
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Default)]
/// Represents a piece of javascript for execution.
///
/// Can be loaded from data at runtime, with `Module::new`, or from a file with `Module::load`.
///
/// It can also be loaded statically with `Module::new_static` or `module!`
pub struct Module {
    filename: MaybePathBuf<'static>,
    contents: Cow<'static, str>,
}

impl<'de> Deserialize<'de> for Module {
    fn deserialize<D>(deserializer: D) -> Result<Module, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct OwnedModule {
            filename: PathBuf,
            contents: String,
        }

        let OwnedModule { filename, contents } = OwnedModule::deserialize(deserializer)?;
        Ok(Module::new(filename, contents))
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.filename().display())
    }
}

impl Module {
    /// Creates a new `Module` instance with the given filename and contents.
    ///
    /// If filename is relative it will be resolved to the current working dir at runtime
    ///
    /// # Arguments
    /// * `filename` - A string representing the filename of the module.
    /// * `contents` - A string containing the contents of the module.
    ///
    /// # Returns
    /// A new `Module` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::Module;
    ///
    /// let module = Module::new("module.js", "console.log('Hello, World!');");
    /// ```
    #[must_use]
    pub fn new(filename: impl AsRef<Path>, contents: impl ToString) -> Self {
        let filename = MaybePathBuf::Owned(filename.as_ref().to_path_buf());
        let contents = Cow::Owned(contents.to_string());

        Self { filename, contents }
    }

    /// Creates a new `Module` instance with the given filename and contents.  
    /// The function is const, and the filename and contents are static strings.
    ///
    /// If filename is relative it will be resolved to the current working dir at runtime
    ///
    /// # Arguments
    /// * `filename` - A string representing the filename of the module.
    /// * `contents` - A string containing the contents of the module.
    ///
    /// # Returns
    /// A new `Module` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::Module;
    ///
    /// let module = Module::new("module.js", "console.log('Hello, World!');");
    /// ```
    #[must_use]
    pub const fn new_static(filename: &'static str, contents: &'static str) -> Self {
        Self {
            filename: MaybePathBuf::new_str(filename),
            contents: Cow::Borrowed(contents),
        }
    }

    /// Loads a `Module` instance from a file with the given filename.
    ///
    /// # Arguments
    /// * `filename` - A string representing the filename of the module file.
    ///
    /// # Returns
    /// A `Result` containing the loaded `Module` instance or an `std::io::Error` if there
    /// are issues reading the file.
    ///
    /// # Errors
    /// Will return an error if the file cannot be read.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::Module;
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
    /// let module = Module::load("src/ext/rustyscript/rustyscript.js")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load(filename: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let contents = read_to_string(filename.as_ref())?;
        Ok(Self::new(filename, &contents))
    }

    /// Attempt to load all `.js`/`.ts` files in a given directory
    ///
    /// Fails if any of the files cannot be loaded
    ///
    /// # Arguments
    /// * `directory` - A string representing the target directory
    ///
    /// # Returns
    /// A `Result` containing a vec of loaded `Module` instances or an `std::io::Error` if there
    /// are issues reading a file.
    ///
    /// # Errors
    /// Will return an error if the directory cannot be read, or if any contained file cannot be read.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::Module;
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
    /// let all_modules = Module::load_dir("src/ext/rustyscript")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_dir(directory: impl AsRef<Path>) -> Result<Vec<Self>, std::io::Error> {
        let mut files: Vec<Self> = Vec::new();
        for file in read_dir(directory)? {
            let file = file?;
            if let Some(filename) = file.path().to_str() {
                // Skip non-js files
                let extension = Path::new(&filename)
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap_or_default();
                if !["js", "ts"].contains(&extension) {
                    continue;
                }

                files.push(Self::load(filename)?);
            }
        }

        Ok(files)
    }

    /// Returns the filename of the module.
    ///
    /// # Returns
    /// A reference to a string containing the filename.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::Module;
    ///
    /// let module = Module::new("module.js", "console.log('Hello, World!');");
    /// println!("Filename: {:?}", module.filename());
    /// ```
    #[must_use]
    pub fn filename(&self) -> &Path {
        self.filename.as_ref()
    }

    /// Returns the contents of the module.
    ///
    /// # Returns
    /// A reference to a string containing the module contents.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::Module;
    ///
    /// let module = Module::new("module.js", "console.log('Hello, World!');");
    /// println!("Module Contents: {}", module.contents());
    /// ```
    #[must_use]
    pub fn contents(&self) -> &str {
        &self.contents
    }
}

#[cfg(test)]
mod test_module {
    use super::*;

    #[test]
    fn test_new_module() {
        let module = Module::new("module.js", "console.log('Hello, World!');");
        assert_eq!(module.filename().to_str().unwrap(), "module.js");
        assert_eq!(module.contents(), "console.log('Hello, World!');");
    }

    #[test]
    fn test_load_module() {
        let module =
            Module::load("src/ext/rustyscript/rustyscript.js").expect("Failed to load module");
        assert_eq!(
            module.filename().to_str().unwrap(),
            "src/ext/rustyscript/rustyscript.js"
        );
    }

    #[test]
    fn test_load_dir() {
        let modules =
            Module::load_dir("src/ext/rustyscript").expect("Failed to load modules from directory");
        assert!(!modules.is_empty());
    }
}
