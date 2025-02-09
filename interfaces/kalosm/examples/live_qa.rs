use surrealdb::{engine::local::RocksDb, Surreal};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    use futures_util::StreamExt;
    use kalosm::audio::*;
    use kalosm::language::*;
    use kalosm::*;
    use std::sync::Arc;
    use tokio::time::{Duration, Instant};

    // Set up the whisper model
    let model = WhisperBuilder::default()
        .with_source(WhisperSource::MediumEn)
        .build()?;

    // Create database connection
    let db = Surreal::new::<RocksDb>("./db/temp.db").await.unwrap();

    // Select a specific namespace / database
    db.use_ns("test").use_db("test").await.unwrap();

    // Create a new document database table
    let document_table = Arc::new(
        db.document_table_builder("documents")
            // Store the embedding database at ./db/embeddings.db
            .at("./db/embeddings.db")
            .build()
            .unwrap(),
    );

    // Record 30s snippets of audio and transcribe them
    {
        let document_table = document_table.clone();
        std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let recording_time = Duration::from_secs(30);
                    loop {
                        let input = MicInput::default()
                            .record_until(Instant::now() + recording_time)
                            .await
                            .unwrap();

                        if let Ok(mut transcribed) = model.transcribe(input) {
                            while let Some(transcribed) = transcribed.next().await {
                                if transcribed.probability_of_no_speech() < 0.90 {
                                    let document =
                                        transcribed.text().into_document().await.unwrap();
                                    document_table.insert(document).await.unwrap();
                                }
                            }
                        }
                    }
                })
        });
    }

    // Create a llama chat model
    let mut model = Llama::new_chat();
    let mut chat = Chat::builder(&mut model).with_system_prompt("The assistant help answer questions based on the context given by the user. The model knows that the information the user gives it is always true.").build();

    loop {
        // Ask the user for a question
        let user_question = prompt_input("\n> ").unwrap();

        // Search for relevant context in the document engine
        let context = document_table
            .select_nearest(&user_question, 5)
            .await?
            .into_iter()
            .map(|document| {
                format!(
                    "Title: {}\nBody: {}\n",
                    document.record.title(),
                    document.record.body()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Format a prompt with the question and context
        let prompt = format!(
            "Here is the relevant context:\n{context}\nGiven that context, answer the following question:\n{user_question}"
        );

        // Display the prompt to the user for debugging purposes
        println!("{}", prompt);

        // And finally, respond to the user
        let output_stream = chat.add_message(prompt).await.unwrap();
        print!("Bot: ");
        output_stream.to_std_out().await.unwrap();
    }
}
