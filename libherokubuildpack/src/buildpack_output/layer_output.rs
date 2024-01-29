//! TODO docs
use crate::buildpack_output::{state, BuildpackOutput, Stream};

pub trait LayerOutput<W>
where
    W: std::io::Write + std::fmt::Debug + Send + Sync + 'static,
{
    fn get(&mut self) -> BuildpackOutput<state::Section, W>;
    fn set(&mut self, output: BuildpackOutput<state::Section, W>);

    fn step(&mut self, s: impl AsRef<str>) {
        let out = self.get().step(s.as_ref());
        self.set(out);
    }

    fn step_stream<T>(&mut self, s: impl AsRef<str>, f: impl FnOnce(&mut Stream<W>) -> T) -> T {
        let mut stream = self.get().step_timed_stream(s.as_ref());
        let out = f(&mut stream);
        let buildpack_output = stream.finish_timed_stream();
        self.set(buildpack_output);
        out
    }

    fn warning(&mut self, s: impl AsRef<str>) {
        let output = self.get();
        let output = output.warning(s.as_ref());
        self.set(output);
    }

    fn important(&mut self, s: impl AsRef<str>) {
        let output = self.get();
        let output = output.important(s.as_ref());
        self.set(output);
    }

    // Intentionally not implemented because it consumes
    // If you want to emit an error inside a layer, raise an error
    // fn error
}
