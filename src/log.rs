
/// Log a message if verbosity is enabled.
macro_rules! vlog {
    ($verbosity_enabled:expr => $fmt:expr $(, $arg:expr )*) => {
        log!("[log]", YELLOW, $verbosity_enabled => $fmt $(, $arg )* );
    }
}

/// Logs an information message unconditionally.
macro_rules! ilog {
    ($fmt:expr $(, $arg:expr )*) => {
        log!("[info]", BRIGHT_BLUE, true => $fmt $(, $arg )* );
    }
}

/// Logs a warning message unconditionally.
macro_rules! warn {
    ($fmt:expr $(, $arg:expr )*) => {
        log!("warning", MAGENTA, true => $fmt $(, $arg )* );
    }
}

/// Log an error message and terminate the process.
macro_rules! fatal {
    ($fmt:expr $(, $arg:expr )*) => {
        {
            log!("error", RED, true => $fmt $(, $arg )* );
            ::std::process::exit(1);
        }
    }
}

/// Write a generic log message to standard error.
macro_rules! log {
    ($label:expr, $color:ident,
     $logging_enabled:expr => $fmt:expr $(, $arg:expr )*) => {
        if $logging_enabled {
            use term;

            let mut t = term::stderr().unwrap();

            t.fg(term::color::$color).ok();
            eprint!("{}: ", $label);
            t.reset().ok();
            eprintln!($fmt $( , $arg )*);
        }
    }
}

