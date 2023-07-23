pub type WasiCtx = u32;

// use alloc::{vec::Vec, string::String, boxed::Box, sync::Arc};
// use rand::RngCore;
// use spin::Mutex;

// #[derive(Clone)]
// pub struct WasiCtx(Arc<WasiCtxInner>);

// pub struct WasiCtxInner {
//     pub args: Vec<String>,
//     pub env: Vec<String>,
//     pub random: Mutex<Box<dyn RngCore + Send + Sync>>,
//     pub clocks: WasiClocks,
//     pub sched: Box<dyn WasiSched>,
//     pub table: Table,
// }

// impl WasiCtx {
//     pub fn new(
//         random: Box<dyn RngCore + Send + Sync>,
//         clocks: WasiClocks,
//         sched: Box<dyn WasiSched>,
//         table: Table,
//     ) -> Self {
//         let s = WasiCtx(Arc::new(WasiCtxInner {
//             args: Vec::new(),
//             env: Vec::new(),
//             random: Mutex::new(random),
//             clocks,
//             sched,
//             table,
//         }));
//         s.set_stdin(Box::new(crate::pipe::ReadPipe::new(io::empty())));
//         s.set_stdout(Box::new(crate::pipe::WritePipe::new(io::sink())));
//         s.set_stderr(Box::new(crate::pipe::WritePipe::new(io::sink())));
//         s
//     }

//     pub fn insert_file(&self, fd: u32, file: Box<dyn WasiFile>, access_mode: FileAccessMode) {
//         self.table()
//             .insert_at(fd, Arc::new(FileEntry::new(file, access_mode)));
//     }

//     pub fn push_file(
//         &self,
//         file: Box<dyn WasiFile>,
//         access_mode: FileAccessMode,
//     ) -> Result<u32, Error> {
//         self.table()
//             .push(Arc::new(FileEntry::new(file, access_mode)))
//     }

//     pub fn insert_dir(&self, fd: u32, dir: Box<dyn WasiDir>, path: PathBuf) {
//         self.table()
//             .insert_at(fd, Arc::new(DirEntry::new(Some(path), dir)));
//     }

//     pub fn push_dir(&self, dir: Box<dyn WasiDir>, path: PathBuf) -> Result<u32, Error> {
//         self.table().push(Arc::new(DirEntry::new(Some(path), dir)))
//     }

//     pub fn table(&self) -> &Table {
//         &self.table
//     }

//     pub fn table_mut(&mut self) -> Option<&mut Table> {
//         Arc::get_mut(&mut self.0).map(|c| &mut c.table)
//     }

//     pub fn push_arg(&mut self, arg: &str) -> Result<(), StringArrayError> {
//         let s = Arc::get_mut(&mut self.0).expect(
//             "`push_arg` should only be used during initialization before the context is cloned",
//         );
//         s.args.push(arg.to_owned())
//     }

//     pub fn push_env(&mut self, var: &str, value: &str) -> Result<(), StringArrayError> {
//         let s = Arc::get_mut(&mut self.0).expect(
//             "`push_env` should only be used during initialization before the context is cloned",
//         );
//         s.env.push(format!("{}={}", var, value))?;
//         Ok(())
//     }

//     pub fn set_stdin(&self, f: Box<dyn WasiFile>) {
//         self.insert_file(0, f, FileAccessMode::READ);
//     }

//     pub fn set_stdout(&self, f: Box<dyn WasiFile>) {
//         self.insert_file(1, f, FileAccessMode::WRITE);
//     }

//     pub fn set_stderr(&self, f: Box<dyn WasiFile>) {
//         self.insert_file(2, f, FileAccessMode::WRITE);
//     }

//     pub fn push_preopened_dir(
//         &self,
//         dir: Box<dyn WasiDir>,
//         path: impl AsRef<Path>,
//     ) -> Result<(), Error> {
//         self.table()
//             .push(Arc::new(DirEntry::new(Some(path.as_ref().to_owned()), dir)))?;
//         Ok(())
//     }
// }

// impl Deref for WasiCtx {
//     type Target = WasiCtxInner;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }