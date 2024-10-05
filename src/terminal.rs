use std::io::{stdout, Write};

// ANSI codes
const ESCAPE: char = 27 as char;
const ALERT: char = 7 as char;

/// Move cursor to beginning of the previous line.
pub fn previous_line() {
  print!("{ESCAPE}[F");
}

/// Clear the current line of all characters.
pub fn clear_line() {
  // move the cursor to the beginning of the line
  print!("\r");

  // print whitespace characters to clear the line
  for _ in 0..get_width() {
    print!(" ");
  }

  // reset back to beginning of line
  print!("\r");
}

/// Enables/disables cursor visibility in the terminal.
pub fn set_cursor_visible(visible: bool) {
  if visible {
    print!("{ESCAPE}[?25h");
  } else {
    print!("{ESCAPE}[?25l");
  }

  stdout().flush().unwrap();
}

/// Sets virtual terminal progress
pub fn progress(progress: u32) {
  print!("{ESCAPE}]9;4;1;{progress}{ALERT}");
}

/// Hide virtual terminal progress
pub fn hide_progress() {
  print!("{ESCAPE}]9;4;0;100{ALERT}");
}

/// Get the ANSI code to color the foreground in `red`, `green`, `blue`.
pub fn ansi_rgb(red: u8, green: u8, blue: u8) -> String {
  format!("{ESCAPE}[38;2;{red};{green};{blue}m")
}

/// Get the terminal's column count.
pub fn get_width() -> u16 {
  termsize::get().unwrap_or(termsize::Size { rows: 10, cols: 80 }).cols
}
