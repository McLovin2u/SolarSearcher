# SolarSearcher

SolarSearcher is a Rust-based tool designed to scrape https://SecondSol.com, a vendor of solar power equipment. It fetches photovoltaic panels, filters them by specified zip codes, and sorts the results based on price per watt.

## Installation
Be sure to have Rust installed: https://www.rust-lang.org/tools/install  
To install SolarSearcher, clone this repository and build the project using Cargo, Rust's package manager.

```bash
git clone https://github.com/yourusername/SolarSearcher.git
cd SolarSearcher
cargo build --release
```
After that you will find the executable in `SolarSearcher/build/release`

## Usage

SolarSearcher offers a set of commands via its command-line interface (CLI):

### Commands

- `filter`: Filters the current database using zip code filters from `config.txt`.
- `clear-local`: Deletes the current database.
- `pull-all`: Checks all pages for articles, updates the database, and deletes articles that are no longer available. (This will take a while)
- `pull`: Checks a specified number of pages for the most recent articles, updates the database (does not delete unavailable articles).
- `help`: Prints the help message or the help for a specific subcommand.

### First Usage

```bash
# Get all articles, this will take a while
./SolarSearcher pull-all

# Filter by zip codes in config.txt
./SolarSearcher filter

# Get the newest articles. This will be much quicker
./SolarSearcher pull [number_of_pages]
```


## Configuration File (`config.txt`)

SolarSearcher utilizes a `config.txt` file to manage settings for scraping and filtering. Ensure the file is in the directory of the executable and follows this JSON structure:

```json
{
    "cookie": "ebizuid_ebiztrader_hash=fasdifjhjasdhfaskdhf; ebizuid_ebiztrader_uid=345; ebiztrader=sdfgd873428jkh; cookies_consent=1",
    "zipcodes": [
        [47000, 57000],
        [87000, 97000]
    ]
}
```

### Fields

- **`cookie`**: This field contains the necessary authentication or session information for accessing SecondSol's data. See below how to obtain this cookie
  
- **`zipcodes`**: A list of valid zip code intervals used for filtering the scraped data. Each entry in the list represents a range of zip codes. SolarSearcher filters the results based on these specified zip code intervals.
- You can use this page to find the relevant zip codes for you: https://www.venue.de/plzkarte/
### Cookies

To ensure that SolarSearcher can scrape the necessary location data you need an account on SecondSol. Then we use the cookie that your browser saves in order to authenticate SolarSearcher.
To do this you can largely follow the guide provided in the second answer provided here: https://stackoverflow.com/questions/23102833/how-to-scrape-a-website-which-requires-login-using-python-and-beautifulsoup
On https://curlconverter.com/ be sure to select Rust. Then just copy the line after ```headers.insert(header::COOKIE``` into the config file.

---
## Contributing

Contributions are welcome! If you'd like to contribute to SolarSearcher, please fork the repository and submit a pull request. 

## License

This project is licensed under the MIT License - see the [LICENSE](LICENCE.txt) file for details.
