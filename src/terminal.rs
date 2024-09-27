/// Move cursor to beginning of the previous line.
pub fn previous_line() {
  print!("{}[F", 27 as char);
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

/// Get the ANSI code to color the foreground in `red`, `green`, `blue`.
pub fn ansi_rgb(red: u8, green: u8, blue: u8) -> String {
  format!("{}[38;2;{red};{green};{blue}m", 27 as char)
}

/// Get the terminal's column count.
pub fn get_width() -> u16 {
  termsize::get().unwrap_or(termsize::Size { rows: 10, cols: 80 }).cols
}
