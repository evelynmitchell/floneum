use kalosm::language::*;

#[tokio::main]
async fn main() {
    let llm = Llama::default();
    let prompt = "Five US states in central US are ";

    println!("# with constraints");
    print!("{}", prompt);
    let states = [
        "Alabama",
        "Alaska",
        "Arizona",
        "Arkansas",
        "California",
        "Colorado",
        "Connecticut",
        "Delaware",
        "Florida",
        "Georgia",
        "Hawaii",
        "Idaho",
        "Illinois",
        "Indiana",
        "Iowa",
        "Kansas",
        "Kentucky",
        "Louisiana",
        "Maine",
        "Maryland",
        "Massachusetts",
        "Michigan",
        "Minnesota",
        "Mississippi",
        "Missouri",
        "Montana",
        "Nebraska",
        "Nevada",
        "New Hampshire",
        "New Jersey",
        "New Mexico",
        "New York",
        "North Carolina",
        "North Dakota",
        "Ohio",
        "Oklahoma",
        "Oregon",
        "Pennsylvania",
        "Rhode Island",
        "South Carolina",
        "South Dakota",
        "Tennessee",
        "Texas",
        "Utah",
        "Vermont",
        "Virginia",
        "Washington",
        "West Virginia",
        "Wisconsin",
        "Wyoming",
    ];
    let states_parser = states
        .into_iter()
        .map(LiteralParser::from)
        .collect::<Vec<_>>();

    let index_parser = IndexParser::new(states_parser);

    let validator = index_parser.then(LiteralParser::from(", ")).repeat(1..=5);
    let (stream, result) = llm
        .stream_structured_text(prompt, validator)
        .await
        .unwrap()
        .split();

    stream.to_std_out().await.unwrap();

    println!(
        "\n{:#?}",
        result
            .await
            .unwrap()
            .unwrap()
            .iter()
            .map(|x| states[x.0 .0])
            .collect::<Vec<_>>()
    );

    println!("\n\n# without constraints");
    print!("{}", prompt);

    let stream = llm.stream_text(prompt).with_max_length(100).await.unwrap();
    stream.to_std_out().await.unwrap();
}
