use std::{ffi, fs, io, path::Path};

use ka::{
    ast,
    diagnostic::{Diagnostic, File, Files, Severity},
    intern::Interner,
    ir::lower::Lowerer,
    lex::Tokens,
    parse::{self, Parser},
    runtime::Runtime,
};

fn main() -> io::Result<()> {
    let mut compiler = Compiler::new();

    compiler.add_package("test", "test")?;
    compiler.add_package("std", "std")?;

    compiler.run("test");

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
    ast: Box<[ast::ModuleDef]>,
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

    fn run(mut self, main_package: &str) {
        let main_module = format!("{main_package}:main");

        if !self.modules.iter().any(|module| module.name == main_module) {
            panic!("main module not found");
        }

        let mut lowerer = Lowerer::new(&mut self.emitter);

        for module in self.modules {
            lowerer.add_module(module.name, module.ast);
        }

        let (program, main) = lowerer.finish(&main_module);

        let mut writer = ka::diagnostic::Writer::new(io::stderr(), &self.files);
        writer.write_report(&self.emitter).unwrap();

        if self.emitter.iter().any(|d| d.severity() == Severity::Error) {
            return;
        }

        if let Some(main) = main {
            let _ = ka::mir::lower::lower(&program, main);
            let mut runtime = Runtime::new(&program);
            runtime.run(main);
        }
    }

    fn add_package(&mut self, name: &str, path: impl AsRef<Path>) -> io::Result<()> {
        let name = format!("{name}:");
        self.add_package_dir(&name, path.as_ref())
    }

    fn add_package_path(&mut self, parent: &str, path: &Path) -> io::Result<()> {
        if path.is_file() {
            return self.add_package_file(parent, path);
        }

        if let Some(name) = path.file_name().and_then(ffi::OsStr::to_str)
            && path.is_dir()
        {
            let name = format!("{parent}{name}/");
            self.add_package_dir(&name, path)
        } else {
            Ok(())
        }
    }

    fn add_package_dir(&mut self, parent: &str, path: &Path) -> io::Result<()> {
        for entry in path.read_dir()? {
            let entry = entry?;
            let path = entry.path();
            self.add_package_path(parent, &path)?;
        }

        Ok(())
    }

    fn add_package_file(&mut self, parent: &str, path: &Path) -> io::Result<()> {
        if path.extension() != Some(ffi::OsStr::new("ka")) {
            return Ok(());
        }

        let name = path
            .file_stem()
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
