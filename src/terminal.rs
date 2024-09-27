/// Move cursor to beginning of the previous line
pub fn previous_line() {
  print!("{}[F", 27 as char);
}

pub fn clear_line() {
  // Move the cursor to the beginning of the line
  print!("\r");

  // Print whitespace characters to clear the line
  for _ in 0..get_width() {
    print!(" ");
  }

  print!("\r");
}

pub fn ansi_rgb(red: u8, green: u8, blue: u8) -> String {
  format!("{}[38;2;{red};{green};{blue}m", 27 as char)
}

pub fn get_width() -> u16 {
  termsize::get().unwrap_or(termsize::Size { rows: 10, cols: 80 }).cols
}
