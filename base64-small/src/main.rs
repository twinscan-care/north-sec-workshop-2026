use base64::engine::general_purpose::STANDARD;
use base64::read::DecoderReader;

fn main() {
    let mut decoder = DecoderReader::new(std::io::stdin(), &STANDARD);
    std::io::copy(&mut decoder, &mut std::io::stdout()).unwrap();
}
