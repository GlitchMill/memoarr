# Memoarr

Memoarr is a Rust-based command-line tool that fetches posts from a specified Mastodon account and formats them into a diary-style HTML output. This tool allows users to archive their social media posts in a structured format, providing an easy way to keep a personal diary.

## Features

- Fetch posts from a specified Mastodon user.
- Format posts into a clean and structured HTML output.
- Exclude replies from the output.
- Customizable output file and template.

## Requirements

- Rust (1.60 or higher)
- Cargo (comes with Rust installation)

## Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/glitchmill/memoarr
   cd memoarr
   ```

2. Build the project:

   ```bash
   cargo build
   ```

## Usage

Run the application with the following command:

```bash
cargo run -- <MASTODON_URL> [--output <OUTPUT_FILE>] [--template <TEMPLATE_FILE>]
```

### Arguments

- `<MASTODON_URL>`: The URL of the Mastodon profile (e.g., `https://mastodon.example.com/@username`).
- `--output <OUTPUT_FILE>`: Specify the output HTML file (default: `output.html`).
- `--template <TEMPLATE_FILE>`: Specify the HTML template file (default: `templates/template.html`).

### Example

```bash
cargo run -- https://mastodon.example.com/@username --output diary.html --template templates/template.html
```

## Configuration

You can customize the output by modifying the HTML template file located in the `templates` directory. 

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or features you'd like to add.

## License

This project is licensed under the GPL V3 License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by the desire to archive social media posts in a personal format.