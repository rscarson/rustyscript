use std::ffi::OsStr;
use std::fs::{ read_to_string, read_dir };
use std::path::Path;
use serde::{ Serialize, Deserialize };

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Represents a pice of javascript for execution.
/// Must be ESM formatted
pub struct Script {
    filename: String,
    contents: String
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
    ///
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
    ///
    /// A `Result` containing the loaded `Script` instance or an `std::io::Error` if there
    /// are issues reading the file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::Script;
    /// 
    /// match Script::load("script.js") {
    ///     Ok(script) => {
    ///         // Handle the loaded script here
    ///         println!("Loaded script: {:?}", script);
    ///     }
    ///     Err(error) => {
    ///         // Handle the error here
    ///         eprintln!("Error loading script: {}", error);
    ///     }
    /// }
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
    ///
    /// A `Result` containing a vec of loaded `Script` instances or an `std::io::Error` if there
    /// are issues reading a file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::Script;
    /// 
    /// match Script::load_dir("my_scripts") {
    ///     Ok(scripts) => {
    ///         // Handle the loaded script here
    ///         println!("Loaded scripts: {:?}", scripts);
    ///     }
    ///     Err(error) => {
    ///         // Handle the error here
    ///         eprintln!("Error loading a script: {}", error);
    ///     }
    /// }
    /// ```
    pub fn load_dir(directory: &str) -> Result<Vec<Self>, std::io::Error> {
        let mut files: Vec<Self> = Vec::new();
        for file in read_dir(directory)?.flatten() {
            if let Some(filename) = file.path().to_str() {
                // Skip non-js files
                let extension = Path::new(&filename).extension().and_then(OsStr::to_str).unwrap_or_default();
                if ["js", "ts"].contains(&extension) { continue; }

                files.push(
                    Self::load(filename)?
                );
            }
        }

        Ok(files)
    }

    /// Returns the filename of the script.
    ///
    /// # Returns
    ///
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
    ///
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