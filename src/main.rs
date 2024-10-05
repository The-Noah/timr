use std::{
  fs,
  io::{stdout, Write},
  path::PathBuf,
  process::exit,
  sync::mpsc::channel,
  thread::sleep,
  time::{Duration, Instant},
};

use serde::Deserialize;

mod terminal;

const BAR_UPDATE_INTERVAL: u128 = 16; // milliseconds
const BAR_EMPTY_CHAR: char = '▒';
const BAR_FULL_CHAR: char = '█';

#[derive(Deserialize)]
struct Config {
  profiles: Option<Vec<Profile>>,
}

#[derive(Deserialize)]
struct Profile {
  name: String,
  duration: String,
}

fn main() {
  // encourage control characters on Windows (https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences)
  #[cfg(target_os = "windows")]
  {
    use windows::Win32::System::Console::{GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE};

    unsafe {
      let handle = GetStdHandle(STD_OUTPUT_HANDLE).expect("Failed to get stdout");
      let mut mode: CONSOLE_MODE = CONSOLE_MODE(0);

      GetConsoleMode(handle, &mut mode).expect("Failed to get console mode");

      if !mode.contains(ENABLE_VIRTUAL_TERMINAL_PROCESSING) {
        SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING).expect("Failed to set console mode");
      }
    }
  }

  let args = std::env::args().collect::<Vec<String>>();
  let args = args.split_at(1).1; // remove self from args list

  if args.is_empty() {
    print_help();
    return;
  }

  let mut duration = None;

  for arg in args {
    match arg.as_str() {
      "-v" | "--version" => {
        println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
      }
      "-h" | "--help" => {
        print_help();
        return;
      }
      _ => {
        // first generic argument is duration, any after that causing the phone program to error
        if duration.is_none() {
          duration = Some(arg);
        } else {
          eprintln!("Unknown option: {}", arg);
          println!("Use '{} --help' for more information", env!("CARGO_PKG_NAME"));

          exit(1);
        }
      }
    }
  }

  if duration.is_none() {
    eprintln!("No duration specified");
    exit(1);
  }

  let duration = duration.unwrap();

  if duration.is_empty() {
    unreachable!("Duration must not be empty");
  }

  let duration = match duration.chars().next().unwrap() {
    '0'..='9' => parse_duration(duration),
    _ => {
      let config_path = home_dir().expect("Failed to find user's home directory").join(".config").join("timr.toml");

      if !config_path.exists() {
        eprintln!("$HOME/.config/timr.toml does not exist");
        exit(1);
      }

      let config: Config = toml::from_str(fs::read_to_string(config_path).expect("Failed to read config file").as_str()).expect("Failed to parse config file");

      if config.profiles.is_none() {
        eprint!("Config does not contain any profiles");
        exit(1);
      }

      let profiles = config.profiles.unwrap();

      let profile = profiles.iter().find(|profile| profile.name == *duration);

      if profile.is_none() {
        eprint!("No profile found matching {}", duration);
        exit(1);
      }

      parse_duration(&profile.unwrap().duration)
    }
  };

  let start = Instant::now();
  let end = start + duration;

  // setup ctrl+c handler
  let (exit_tx, exit_rx) = channel();
  ctrlc::set_handler(move || exit_tx.send(()).expect("Could not send signal on channel.")).expect("Error setting Ctrl-C handler");

  terminal::set_cursor_visible(false);

  println!(); // create an empty line, as below we will move up and clear it

  let mut last_update = Instant::now();
  loop {
    if exit_rx.try_recv().is_ok() {
      terminal::clear_line();

      terminal::set_cursor_visible(true);
      println!("Exiting early!");

      stdout().flush().unwrap();

      return;
    }

    let now = Instant::now();

    if now > end {
      break;
    }

    if last_update.elapsed().as_millis() < BAR_UPDATE_INTERVAL {
      sleep(Duration::from_millis((BAR_UPDATE_INTERVAL - last_update.elapsed().as_millis()) as u64));
      continue;
    }

    let bar_width = match terminal::get_width() - 15 {
      n if n < 30 => n,
      _ => 30,
    };

    let progress = now.duration_since(start).as_millis() as f64 / duration.as_millis() as f64; // 0-1
    let progress_width = (progress * bar_width as f64).round() as u16;

    let remaining = end - now;
    let seconds = remaining.as_secs_f64();

    terminal::previous_line();
    terminal::clear_line();

    // print current time (clock)
    print!("{} - ", chrono::Local::now().format("%_I:%M%P").to_string().trim());

    // print hours remaining (if any)
    match (seconds / 3600.0).floor() {
      hours if hours > 0.0 => print!("{}h", hours),
      _ => {}
    };

    // print minutes remaining (if any)
    match ((seconds % 3600.0) / 60.0).floor() {
      minutes if minutes > 0.0 => print!("{}m", minutes),
      _ => {}
    }

    // print seconds remaining
    println!("{}s", (seconds % 60.0).floor());

    terminal::clear_line();

    // print the solid progress bar
    for i in 0..progress_width {
      let red = lerp(90, 123, i as f64 / bar_width as f64);
      let green = lerp(105, 90, i as f64 / bar_width as f64);

      print!("{}{}", terminal::ansi_rgb(red, green, 237), BAR_FULL_CHAR);
    }

    // print empty progress bar and progress percent
    print!(
      "{}{}{}[39m  {}%",
      terminal::ansi_rgb(100, 100, 100),
      BAR_EMPTY_CHAR.to_string().repeat((bar_width - progress_width) as usize),
      27 as char,
      (progress * 100.0).round()
    );

    // output progress for virtual terminals
    terminal::progress((progress * 100.0).round() as u32);

    stdout().flush().unwrap();

    last_update = now;
  }

  terminal::previous_line();
  terminal::clear_line();

  // reset progress bar
  terminal::hide_progress();

  print!("{}", 7 as char); // beep/alert

  terminal::set_cursor_visible(true);

  println!("Finished!");

  terminal::clear_line();
}

fn parse_duration(duration: &str) -> Duration {
  let mut seconds = 0;
  let mut current_number = String::new(); // temporary buffer to store the currently parsing number

  for character in duration.chars() {
    match character {
      // take our current buffer and store it as seconds
      's' => {
        if current_number.is_empty() {
          eprintln!("No number found before seconds");
          exit(1);
        }

        seconds += current_number.parse::<u64>().unwrap();
        current_number = String::new();
      }
      'm' => {
        if current_number.is_empty() {
          eprintln!("No number found before minutes");
          exit(1);
        }

        seconds += current_number.parse::<u64>().unwrap() * 60;
        current_number = String::new();
      }
      'h' => {
        if current_number.is_empty() {
          eprintln!("No number found before hours");
          exit(1);
        }

        seconds += current_number.parse::<u64>().unwrap() * 3600;
        current_number = String::new();
      }

      // append to our buffer
      '0'..='9' => {
        current_number.push(character);
      }

      // invalid character found
      _ => {
        eprintln!("Invalid time!");
        exit(1);
      }
    }
  }

  // if there are any remaining numbers, assume seconds
  if !current_number.is_empty() {
    seconds += current_number.parse::<u64>().unwrap();
  }

  Duration::from_secs(seconds)
}

fn home_dir() -> Option<PathBuf> {
  #[cfg(target_family = "windows")]
  {
    use windows_sys::Win32::{UI::Shell::*, *};

    let mut p = std::ptr::null_mut();
    let r = if unsafe { SHGetKnownFolderPath(&FOLDERID_Profile, 0, std::ptr::null_mut(), &mut p) } == 0 {
      let w = unsafe { core::slice::from_raw_parts(p, Globalization::lstrlenW(p) as _) };
      let o: std::ffi::OsString = std::os::windows::ffi::OsStringExt::from_wide(w);
      Some(o.into())
    } else {
      None
    };

    unsafe { System::Com::CoTaskMemFree(p as _) }
    r
  }

  #[cfg(not(target_family = "windows"))]
  std::env::var_os("HOME").map(Into::into)
}

fn print_help() {
  println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
  println!("Usage: {} [options]", env!("CARGO_PKG_NAME"));
  println!();
  println!("Options:");
  println!("  duration       Start a timer for duration");
  println!("  -v, --version  Print version information");
  println!("  -h, --help     Print this help message");
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
  ((1.0 - t) * (a as f64) + t * (b as f64)).round() as u8
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_default() {
    assert_eq!(parse_duration("1"), Duration::from_secs(1));
    assert_eq!(parse_duration("9"), Duration::from_secs(9));
    assert_eq!(parse_duration("10"), Duration::from_secs(10));

    assert_eq!(parse_duration("1m1"), Duration::from_secs(61));
    assert_eq!(parse_duration("1m9"), Duration::from_secs(69));
    assert_eq!(parse_duration("1m10"), Duration::from_secs(70));

    assert_eq!(parse_duration("1h1m1"), Duration::from_secs(3661));
    assert_eq!(parse_duration("1h1m9"), Duration::from_secs(3669));
    assert_eq!(parse_duration("1h1m10"), Duration::from_secs(3670));
  }

  #[test]
  fn parse_full() {
    assert_eq!(parse_duration("1s"), Duration::from_secs(1));
    assert_eq!(parse_duration("9s"), Duration::from_secs(9));
    assert_eq!(parse_duration("10s"), Duration::from_secs(10));

    assert_eq!(parse_duration("1m1s"), Duration::from_secs(61));
    assert_eq!(parse_duration("1m9s"), Duration::from_secs(69));
    assert_eq!(parse_duration("1m10s"), Duration::from_secs(70));

    assert_eq!(parse_duration("1h1m1s"), Duration::from_secs(3661));
    assert_eq!(parse_duration("1h1m9s"), Duration::from_secs(3669));
    assert_eq!(parse_duration("1h1m10s"), Duration::from_secs(3670));
  }

  #[test]
  fn parse_seconds() {
    assert_eq!(parse_duration("1s"), Duration::from_secs(1));
    assert_eq!(parse_duration("9s"), Duration::from_secs(9));
    assert_eq!(parse_duration("19s"), Duration::from_secs(19));
    assert_eq!(parse_duration("61s"), Duration::from_secs(61));
  }

  #[test]
  fn parse_minutes() {
    assert_eq!(parse_duration("1m"), Duration::from_secs(60));
    assert_eq!(parse_duration("9m"), Duration::from_secs(540));
    assert_eq!(parse_duration("19m"), Duration::from_secs(1140));
    assert_eq!(parse_duration("61m"), Duration::from_secs(3660));
  }

  #[test]
  fn parse_hours() {
    assert_eq!(parse_duration("1h"), Duration::from_secs(3600));
    assert_eq!(parse_duration("9h"), Duration::from_secs(32400));
    assert_eq!(parse_duration("19h"), Duration::from_secs(68400));
    assert_eq!(parse_duration("61h"), Duration::from_secs(219600));
  }
}
