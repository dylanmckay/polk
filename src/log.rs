
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

/// Prints out a fatal `Error` and an explanation message then
/// terminates the process.
macro_rules! fatal_error {
    ($error:expr) => {
        {
            let error: $crate::Error = $error.into();
            let errors: Vec<_> = error.iter().map(ToString::to_string).collect();

            /// Print pretty error messages in release builds.
            #[cfg(not(debug_assertions))]
            {
                fatal!("{}", error)
            }

            /// Print useful stacktrace in debug mode.
            #[cfg(debug_assertions)]
            {
                eprintln!("error: {}", errors.join(" - "));
                panic!("{}", error);
            }
        }
    };

    ($error:expr, $message:expr) => {
        {
            use $crate::ResultExt;

            // We can only chain `Result`, not `Error`.
            let error: Result<(), $crate::Error> = Err($error.into());
            let error = error.chain_err(|| $message);
            let error = error.err().unwrap();

            fatal_error!(error);
        }
    };
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

