use candle_core::Device;
use kalosm_language_model::{Embedder, Embedding, VectorSpace};

use crate::{
    Class, ClassificationDataset, ClassificationDatasetBuilder, Classifier, ClassifierConfig,
};

/// A builder for [`TextClassifier`].
pub struct TextClassifierDatasetBuilder<'a, T: Class, E: Embedder> {
    dataset: ClassificationDatasetBuilder<T>,
    embedder: &'a mut E,
}

impl<'a, T: Class, E: Embedder> TextClassifierDatasetBuilder<'a, T, E> {
    /// Creates a new [`TextClassifierDatasetBuilder`].
    pub fn new(embedder: &'a mut E) -> Self {
        Self {
            dataset: ClassificationDatasetBuilder::new(),
            embedder,
        }
    }

    /// Adds a new example to the dataset.
    pub async fn add(&mut self, text: &str, class: T) -> anyhow::Result<()> {
        let embedding = self.embedder.embed(text).await?;
        self.dataset.add(embedding.to_vec(), class);
        Ok(())
    }

    /// Builds the dataset.
    pub fn build(self, device: &Device) -> candle_core::Result<ClassificationDataset> {
        self.dataset.build(device)
    }
}

/// A text classifier.
///
/// # Example
///
/// ```rust
/// use kalosm_learning::{Class, Classifier, ClassifierConfig};
/// use candle_core::Device;
/// use kalosm_language_model::Embedder;
/// use rbert::{Bert, BertSpace};
///
/// #[tokio::test]
/// async fn simplified() -> anyhow::Result<()> {
///     use crate::{Class, Classifier, ClassifierConfig};
///     use candle_core::Device;
///     use kalosm_language_model::Embedder;
///     use rbert::{Bert, BertSpace};
///
///     #[derive(Debug, Copy, Clone, PartialEq, Eq, Class)]
///     enum MyClass {
///         Person,
///         Thing,
///     }
///
///     let mut bert = Bert::builder().build()?;
///
///     let dev = Device::cuda_if_available(0)?;
///     let person_questions = vec![
///         "What is the author's name?",
///         "What is the author's age?",
///         "Who is the queen of England?",
///         "Who is the president of the United States?",
///         "Who is the president of France?",
///         "Tell me about the CEO of Apple.",
///         "Who is the CEO of Google?",
///         "Who is the CEO of Microsoft?",
///         "What person invented the light bulb?",
///         "What person invented the telephone?",
///         "What is the name of the person who invented the light bulb?",
///         "Who wrote the book 'The Lord of the Rings'?",
///         "Who wrote the book 'The Hobbit'?",
///         "How old is the author of the book 'The Lord of the Rings'?",
///         "How old is the author of the book 'The Hobbit'?",
///         "Who is the best soccer player in the world?",
///         "Who is the best basketball player in the world?",
///         "Who is the best tennis player in the world?",
///         "Who is the best soccer player in the world right now?",
///         "Who is the leader of the United States?",
///         "Who is the leader of France?",
///         "What is the name of the leader of the United States?",
///         "What is the name of the leader of France?",
///     ];
///     let thing_sentences = vec![
///         "What is the capital of France?",
///         "What is the capital of England?",
///         "What is the name of the biggest city in the world?",
///         "What tool do you use to cut a tree?",
///         "What tool do you use to cut a piece of paper?",
///         "What is a good book to read?",
///         "What is a good movie to watch?",
///         "What is a good song to listen to?",
///         "What is the best tool to use to create a website?",
///         "What is the best tool to use to create a mobile app?",
///         "How long does it take to fly from Paris to New York?",
///         "How do you make a cake?",
///         "How do you make a pizza?",
///         "How can you make a website?",
///         "What is the best way to learn a new language?",
///         "What is the best way to learn a new programming language?",
///         "What is a framework?",
///         "What is a library?",
///         "What is a good way to learn a new language?",
///         "What is a good way to learn a new programming language?",
///         "What is the city with the most people in the world?",
///         "What is the most spoken language in the world?",
///         "What is the most spoken language in the United States?",
///     ];
///
///     let mut dataset = TextClassifierDatasetBuilder::<MyClass, _, _>::new(&mut bert);
///
///     for question in &person_questions {
///         dataset.add(question, MyClass::Person).await?;
///     }
///
///     for sentence in &thing_sentences {
///         dataset.add(sentence, MyClass::Thing).await?;
///     }
///
///     let dataset = dataset.build(&dev)?;
///
///     let mut classifier;
///     let layers = vec![5, 8, 5];
///
///     loop {
///         classifier = TextClassifier::<MyClass, BertSpace>::new(Classifier::new(
///             &dev,
///             ClassifierConfig::new(384).layers_dims(layers.clone()),
///         )?);
///         if let Err(error) = classifier.train(&dataset, &dev, 100) {
///             println!("Error: {:?}", error);
///         } else {
///             break;
///         }
///         println!("Retrying...");
///     }
///
///     let config = classifier.config();
///     classifier.save("classifier.safetensors")?;
///     let mut classifier = Classifier::<MyClass>::load("classifier.safetensors", &dev, config)?;
///
///     let tests = [
///         "Who is the president of Russia?",
///         "What is the capital of Russia?",
///         "Who invented the TV?",
///         "What is the best way to learn a how to ride a bike?",
///     ];
///
///     for test in &tests {
///         let input = bert.embed(test).await?.to_vec();
///         let class = classifier.run(&input)?;
///         println!();
///         println!("{test}");
///         println!("{:?} {:?}", &input[..5], class);
///     }
///
///     Ok(())
/// }
/// ```
pub struct TextClassifier<T: Class, S: VectorSpace + Send + Sync + 'static> {
    model: Classifier<T>,
    phantom: std::marker::PhantomData<S>,
}

impl<T: Class, S: VectorSpace + Send + Sync + 'static> TextClassifier<T, S> {
    /// Creates a new [`TextClassifier`].
    pub fn new(model: Classifier<T>) -> Self {
        Self {
            model,
            phantom: std::marker::PhantomData,
        }
    }

    /// Runs the classifier on the given input.
    pub fn run(&mut self, input: Embedding<S>) -> candle_core::Result<T> {
        self.model.run(&input.to_vec())
    }

    /// Trains the classifier on the given dataset.
    pub fn train(
        &mut self,
        dataset: &ClassificationDataset,
        device: &Device,
        epochs: usize,
    ) -> anyhow::Result<f32> {
        self.model.train(dataset, device, epochs)
    }

    /// Get the configuration of the classifier.
    pub fn config(&self) -> ClassifierConfig {
        self.model.config()
    }

    /// Saves the classifier to the given path.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> candle_core::Result<()> {
        self.model.save(path)
    }

    /// Loads a classifier from the given path.
    pub fn load<P: AsRef<std::path::Path>>(
        path: P,
        device: &Device,
        config: ClassifierConfig,
    ) -> candle_core::Result<Self> {
        let model = Classifier::load(path, device, config)?;
        Ok(Self::new(model))
    }
}

#[tokio::test]
async fn simplified() -> anyhow::Result<()> {
    use crate::{Class, Classifier, ClassifierConfig};
    use candle_core::Device;
    use kalosm_language_model::Embedder;
    use rbert::{Bert, BertSpace};

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Class)]
    enum MyClass {
        Person,
        Thing,
    }

    let mut bert = Bert::builder().build()?;

    let dev = Device::cuda_if_available(0)?;
    let person_questions = vec![
        "What is the author's name?",
        "What is the author's age?",
        "Who is the queen of England?",
        "Who is the president of the United States?",
        "Who is the president of France?",
        "Tell me about the CEO of Apple.",
        "Who is the CEO of Google?",
        "Who is the CEO of Microsoft?",
        "What person invented the light bulb?",
        "What person invented the telephone?",
        "What is the name of the person who invented the light bulb?",
        "Who wrote the book 'The Lord of the Rings'?",
        "Who wrote the book 'The Hobbit'?",
        "How old is the author of the book 'The Lord of the Rings'?",
        "How old is the author of the book 'The Hobbit'?",
        "Who is the best soccer player in the world?",
        "Who is the best basketball player in the world?",
        "Who is the best tennis player in the world?",
        "Who is the best soccer player in the world right now?",
        "Who is the leader of the United States?",
        "Who is the leader of France?",
        "What is the name of the leader of the United States?",
        "What is the name of the leader of France?",
    ];
    let thing_sentences = vec![
        "What is the capital of France?",
        "What is the capital of England?",
        "What is the name of the biggest city in the world?",
        "What tool do you use to cut a tree?",
        "What tool do you use to cut a piece of paper?",
        "What is a good book to read?",
        "What is a good movie to watch?",
        "What is a good song to listen to?",
        "What is the best tool to use to create a website?",
        "What is the best tool to use to create a mobile app?",
        "How long does it take to fly from Paris to New York?",
        "How do you make a cake?",
        "How do you make a pizza?",
        "How can you make a website?",
        "What is the best way to learn a new language?",
        "What is the best way to learn a new programming language?",
        "What is a framework?",
        "What is a library?",
        "What is a good way to learn a new language?",
        "What is a good way to learn a new programming language?",
        "What is the city with the most people in the world?",
        "What is the most spoken language in the world?",
        "What is the most spoken language in the United States?",
    ];

    let mut dataset = TextClassifierDatasetBuilder::<MyClass, _>::new(&mut bert);

    for question in &person_questions {
        dataset.add(question, MyClass::Person).await?;
    }

    for sentence in &thing_sentences {
        dataset.add(sentence, MyClass::Thing).await?;
    }

    let dataset = dataset.build(&dev)?;

    let mut classifier;
    let layers = vec![5, 8, 5];

    loop {
        classifier = TextClassifier::<MyClass, BertSpace>::new(Classifier::new(
            &dev,
            ClassifierConfig::new(384).layers_dims(layers.clone()),
        )?);
        if let Err(error) = classifier.train(&dataset, &dev, 100) {
            println!("Error: {:?}", error);
        } else {
            break;
        }
        println!("Retrying...");
    }

    let config = classifier.model.config();
    classifier.save("classifier.safetensors")?;
    let mut classifier = Classifier::<MyClass>::load("classifier.safetensors", &dev, config)?;

    let tests = [
        "Who is the president of Russia?",
        "What is the capital of Russia?",
        "Who invented the TV?",
        "What is the best way to learn a how to ride a bike?",
    ];

    for test in &tests {
        let input = bert.embed(test).await?.to_vec();
        let class = classifier.run(&input)?;
        println!();
        println!("{test}");
        println!("{:?} {:?}", &input[..5], class);
    }

    Ok(())
}
