use std::fs::read_to_string;

#[derive(Clone)]
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

    /// Returns the filename of the script.
    ///
    /// # Returns
    ///
    /// A reference to a string containing the filename.
    ///
    /// # Example
    ///
    /// ```rust
    /// let script = Script::new("script.js", "console.log('Hello, World!');");
    /// let filename = script.filename();
    /// println!("Filename: {}", filename);
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
    /// let script = Script::new("script.js", "console.log('Hello, World!');");
    /// let contents = script.contents();
    /// println!("Script Contents: {}", contents);
    /// ```
    pub fn contents(&self) -> &str {
        &self.contents
    }
}