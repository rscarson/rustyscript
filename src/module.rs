use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};

/// A static representation of a module
/// use `.to_module()` to get a module instance to use with a runtime
pub struct StaticModule(&'static str, &'static str);
impl StaticModule {
    /// Create a new `StaticModule`
    /// use the module!(filename, contents) macro instead!
    #[must_use]
    pub const fn new(filename: &'static str, contents: &'static str) -> Self {
        Self(filename, contents)
    }

    /// Get an instance of this `StaticModule` that can be used with a runtime
    #[must_use]
    pub fn to_module(&self) -> Module {
        Module::new(Path::new(self.0), self.1)
    }
}

/// Creates a static module
///
/// # Arguments
/// * `filename` - A string representing the filename of the module.
/// * `contents` - A string containing the contents of the module.
///
/// # Example
///
/// ```rust
/// use rustyscript::{ module, StaticModule };
///
/// const MY_SCRIPT: StaticModule = module!(
///     "filename.js",
///     "export const myValue = 42;"
/// );
///
/// let module_instance = MY_SCRIPT.to_module();
/// ```
#[macro_export]
macro_rules! module {
    ($filename:literal, $contents:literal) => {
        StaticModule::new($filename, $contents)
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
        StaticModule::new($filename, include_str!($filename))
    };
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
/// Represents a pice of javascript for execution.
pub struct Module {
    filename: PathBuf,
    contents: String,
}

impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.filename().display())
    }
}

impl Module {
    /// Creates a new `Module` instance with the given filename and contents.
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
    pub fn new(filename: impl AsRef<Path>, contents: &str) -> Self {
        Self {
            filename: filename.as_ref().to_path_buf(),
            contents: contents.to_string(),
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
        &self.filename
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
