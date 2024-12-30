use std::{fmt::Debug, hash::Hash};

use itertools::Itertools;

pub type RandomTestingResult = Result<(), RandomTestingFail>;

pub fn random_testing<F, E>(function: F, sample_size: u64, nb_max_err: usize) -> RandomTestingResult
where
    F: Fn(u64) -> Result<(), E>,
    E: Debug + 'static,
{
    let errors: Vec<E> = (0..sample_size)
        .filter_map(move |rng| function(rng).err())
        .collect();
    let taille = errors.len();
    let error_wrapper_opt = errors
        .into_iter()
        .map(|e| ErrorWrapper(e))
        .counts()
        .into_iter()
        .max_by(|(_, a), (_, b)| a.cmp(b));
    let (la_plus_presente, la_plus_presente_nb) = match error_wrapper_opt {
        Some((err, nb)) => (Some(err.0), nb),
        None => (None, 0),
    };
    if taille > nb_max_err {
        Err(RandomTestingFail {
            expected: nb_max_err,
            actual: taille,
            la_plus_presente_nb,
            la_plus_presente: Box::new(la_plus_presente),
        })
    } else {
        dbg!(format!("nombre erreur: {nb_max_err}/{taille}"));
        Ok(())
    }
}

struct ErrorWrapper<E: Debug>(E);

impl<E: Debug> ErrorWrapper<E> {
    fn message(&self) -> String {
        let error = &self.0;
        format!("{error:?}")
    }
}

impl<E: Debug> Hash for ErrorWrapper<E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.message().hash(state);
    }
}

impl<E: Debug> PartialEq for ErrorWrapper<E> {
    fn eq(&self, other: &Self) -> bool {
        self.message() == other.message()
    }
}

impl<E: Debug> Eq for ErrorWrapper<E> {}

#[derive(Debug)]
pub struct RandomTestingFail {
    expected: usize,
    actual: usize,
    la_plus_presente_nb: usize,
    la_plus_presente: Box<dyn Debug>,
}
