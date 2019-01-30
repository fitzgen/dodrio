fn main() {}

#[cfg(test)]
mod tests {
    extern crate compiletest_rs as compiletest;
    use std::path::PathBuf;

    fn run_mode(mode: &'static str) {
        let mut config = self::compiletest::Config::default();

        config.mode = mode.parse().expect("Invalid mode");
        config.src_base = PathBuf::from(format!("{}", mode));

        // Populate config.target_rustcflags with dependencies on the path.
        config.link_deps();
        // If your tests import the parent crate, this helps with E0464
        config.clean_rmeta();

        compiletest::run_tests(&config);
    }

    #[test]
    fn compile_test() {
        run_mode("compile-fail");
    }
}
