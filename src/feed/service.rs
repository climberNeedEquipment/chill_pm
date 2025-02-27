use super::{Feed, Processor};

struct FeedService<T, G> {
    feed: Box<dyn Feed<T>>,
    processors: Box<dyn Processor<T, G>>,
}

impl<T, G> FeedService<T, G> {
    pub fn new(feed: Box<dyn Feed<T>>, processors: Box<dyn Processor<T, G>>) -> Self {
        Self { feed, processors }
    }
}
