use std::{ffi, fs, io, path::Path};

use ka::{
    ast,
    diagnostic::{Diagnostic, File, Files, Severity},
    intern::Interner,
    lex::Tokens,
    lower::Lowerer,
    parse::{self, Parser},
    runtime::Runtime,
};

fn main() -> io::Result<()> {
    let mut compiler = Compiler::new();

    compiler.add_package("test")?;
    compiler.add_package("std")?;

    compiler.run();

    Ok(())
}

struct Compiler {
    emitter: Vec<Diagnostic>,
    files: Files,
    interner: Interner,
    modules: Vec<Module>,
}

struct Module {
    name: &'static str,
    ast: Box<[ast::Def]>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            emitter: Vec::new(),
            files: Files::new(),
            interner: Interner::new(),
            modules: Vec::new(),
        }
    }

    fn run(mut self) {
        let mut lowerer = Lowerer::new(&mut self.emitter);

        for module in self.modules {
            lowerer.add_module(module.name, module.ast);
        }

        let program = lowerer.finish();

        let mut writer = ka::diagnostic::Writer::new(io::stderr(), &self.files);

        for diagnostic in &self.emitter {
            let _ = writer.write(diagnostic);
        }

        if self.emitter.iter().any(|d| d.severity() == Severity::Error) {
            return;
        }

        let mut runtime = Runtime::new(&program);
        runtime.run();
    }

    fn add_package(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = fs::canonicalize(path)?;
        let name = path
            .file_name()
            .and_then(ffi::OsStr::to_str)
            .ok_or_else(|| io::Error::other("invalid package root"))?;

        let name = format!("{name}:");

        self.add_package_dir(&name, &path)
    }

    fn add_package_dir(&mut self, parent: &str, path: &Path) -> io::Result<()> {
        for entry in path.read_dir()? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().and_then(ffi::OsStr::to_str);

                if let Some(name) = name {
                    let name = format!("{parent}{name}/");
                    self.add_package_dir(&name, &path)?;
                }
            } else {
                self.add_package_file(parent, &path)?;
            }
        }

        Ok(())
    }

    fn add_package_file(&mut self, parent: &str, path: &Path) -> io::Result<()> {
        if path.extension() != Some(ffi::OsStr::new("ka")) {
            return Ok(());
        }

        let name = path
            .file_name()
            .and_then(ffi::OsStr::to_str)
            .ok_or_else(|| io::Error::other("invalid `ka` file"))?;

        let name = format!("{parent}{name}");
        let name = self.interner.intern(name);

        let file = self.files.add(File {
            name: name.to_string(),
            path: path.to_path_buf(),
        });

        let input = fs::read_to_string(path)?;

        let tokens = Tokens::lex(&mut self.emitter, &mut self.interner, file, &input);

        let mut parser = Parser::new(&mut self.emitter, &tokens);
        let ast = parse::file(&mut parser).into();

        self.modules.push(Module { name, ast });

        Ok(())
    }
}
