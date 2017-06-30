error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Io(::std::io::Error);
        WalkDir(::walkdir::Error);
        Term(::term::Error);
    }

    errors {
        // FIXME: remove, example code
        InvalidToolchainName(t: String) {
            description("invalid toolchain name")
            display("invalid toolchain name: '{}'", t)
        }
    }
}
