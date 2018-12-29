// Application properties
pub const APP_NAME: &'static str = "pixelpwnr-server";
pub const APP_VERSION: &'static str = "0.1";
pub const APP_AUTHOR: &'static str = "Tim Visee <timvisee@gmail.com>";
pub const APP_ABOUT: &'static str = "Blazingly fast GPU accelerated pixelflut server.";

/// The maximum line length of incomming requests.
/// Lines longer than the specified number of characters will be dropped.
pub const LINE_LENGTH_MAX: usize = 80;
