use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

/// A static representation of a script
/// use `.to_script()` to get a script instance to use with a runtime
pub struct StaticScript(&'static str, &'static str);
impl StaticScript {
    /// Create a new StaticScript
    /// use the script!(filename, contents) macro instead!
    pub const fn new(filename: &'static str, contents: &'static str) -> Self {
        Self(filename, contents)
    }

    /// Get an instance of this StaticScript that can be used with a runtime
    pub fn to_script(&self) -> Script {
        Script::new(self.0, self.1)
    }
}

/// Creates a static script
///
/// # Arguments
/// * `filename` - A string representing the filename of the script.
/// * `contents` - A string containing the contents of the script.
///
/// # Example
///
/// ```rust
/// use js_playground::{ script, StaticScript };
///
/// const MY_SCRIPT: StaticScript = script!(
///     "filename.js",
///     "export const myValue = 42;"
/// );
///
/// let script_instance = MY_SCRIPT.to_script();
/// ```
#[macro_export]
macro_rules! script {
    ($filename:literal, $contents:literal) => {
        StaticScript::new($filename, $contents)
    };
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
/// Represents a pice of javascript for execution.
/// Must be ESM formatted
pub struct Script {
    filename: String,
    contents: String,
}

impl Display for Script {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.filename())
    }
}

impl Script {
    /// Creates a new `Script` instance with the given filename and contents.
    /// If filename is relative it will be resolved to the current working dir at runtime
    ///
    /// # Arguments
    /// * `filename` - A string representing the filename of the script.
    /// * `contents` - A string containing the contents of the script.
    ///
    /// # Returns
    /// A new `Script` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::Script;
    ///
    /// let script = Script::new("script.js", "console.log('Hello, World!');");
    /// ```
    pub fn new(filename: &str, contents: &str) -> Self {
        Self {
            filename: filename.to_string(),
            contents: contents.to_string(),
        }
    }

    /// Loads a `Script` instance from a file with the given filename.
    ///
    /// # Arguments
    /// * `filename` - A string representing the filename of the script file.
    ///
    /// # Returns
    /// A `Result` containing the loaded `Script` instance or an `std::io::Error` if there
    /// are issues reading the file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::Script;
    ///
    /// # fn main() -> Result<(), js_playground::Error> {
    /// let script = Script::load("src/ext/js_playground.js")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load(filename: &str) -> Result<Self, std::io::Error> {
        let contents = read_to_string(filename)?;
        Ok(Self::new(filename, &contents))
    }

    /// Attempt to load all js/ts files in a given directory
    /// Fails if any of the files cannot be loaded
    ///
    /// # Arguments
    /// * `directory` - A string representing the target directory
    ///
    /// # Returns
    /// A `Result` containing a vec of loaded `Script` instances or an `std::io::Error` if there
    /// are issues reading a file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::Script;
    ///
    /// # fn main() -> Result<(), js_playground::Error> {
    /// let all_scripts = Script::load_dir("src/ext")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_dir(directory: &str) -> Result<Vec<Self>, std::io::Error> {
        let mut files: Vec<Self> = Vec::new();
        for file in read_dir(directory)?.flatten() {
            if let Some(filename) = file.path().to_str() {
                // Skip non-js files
                let extension = Path::new(&filename)
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap_or_default();
                if ["js", "ts"].contains(&extension) {
                    continue;
                }

                files.push(Self::load(filename)?);
            }
        }

        Ok(files)
    }

    /// Returns the filename of the script.
    ///
    /// # Returns
    /// A reference to a string containing the filename.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::Script;
    ///
    /// let script = Script::new("script.js", "console.log('Hello, World!');");
    /// println!("Filename: {}", script.filename());
    /// ```
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Returns the contents of the script.
    ///
    /// # Returns
    /// A reference to a string containing the script contents.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::Script;
    ///
    /// let script = Script::new("script.js", "console.log('Hello, World!');");
    /// println!("Script Contents: {}", script.contents());
    /// ```
    pub fn contents(&self) -> &str {
        &self.contents
    }
}

#[cfg(test)]
mod test_script {
    use super::*;

    #[test]
    fn test_new_script() {
        let script = Script::new("script.js", "console.log('Hello, World!');");
        assert_eq!(script.filename(), "script.js");
        assert_eq!(script.contents(), "console.log('Hello, World!');");
    }

    #[test]
    fn test_load_script() {
        let script = Script::load("src/ext/js_playground.js").expect("Failed to load script");
        assert_eq!(script.filename(), "src/ext/js_playground.js");
    }

    #[test]
    fn test_load_dir() {
        let scripts = Script::load_dir("src/ext").expect("Failed to load scripts from directory");
        assert!(scripts.len() > 0);
    }
}
