use std::{fs::File, io::Read};

use solana_client::client_error::reqwest;

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct IpFsPinata {
    pub IpfsHash: String,
    pub PinSize: u64,
    pub Timestamp: String,
    pub isDuplicate: Option<bool>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct JsonMetaData {
    pub name: String,
    pub symbol: String,
    pub image: String,
    pub description: Option<String>,
}

pub const PINATA_API: &str = "";

pub async fn pinata_image_ipfs() -> Result<String, Box<dyn std::error::Error>> {
    let boundary = "--------------------------970379464229125510173661";

    if PINATA_API.is_empty() {
        println!("Unable to create URI");
        return Ok("Invalid".into());
    }

    let mut file = File::open("./test.jpg")?;
    let mut image_data = Vec::new();
    file.read_to_end(&mut image_data)?;

    let mut payload = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.jpg\"\r\nContent-Type: image/jpeg\r\n\r\n",
    )
    .into_bytes();

    payload.extend(image_data);
    payload.extend(format!("\r\n--{boundary}--\r\n").into_bytes());

    let client = reqwest::Client::new();

    let res = client
        .post("https://api.pinata.cloud/pinning/pinFileToIPFS")
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .header("Authorization", format!("Bearer {}", PINATA_API))
        .body(payload)
        .send()
        .await?;

    let body = res.text().await?;
    let body: IpFsPinata = serde_json::from_str(&body)?;

    Ok(body.IpfsHash)
}

pub async fn json_metadata_ipfs(
    bundle: JsonMetaData,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("{:#?}", bundle);
    let client = reqwest::Client::new();

    if PINATA_API.is_empty() {
        println!("Unable to create URI");
        return Ok("Invalid".into());
    }

    let res = client
        .post("https://api.pinata.cloud/pinning/pinJSONToIPFS")
        .header("Authorization", format!("Bearer {}", PINATA_API))
        .json(&bundle)
        .send()
        .await?;

    let body = res.text().await?;

    println!("{:#?}", body);

    let body: IpFsPinata = serde_json::from_str(&body)?;

    Ok(body.IpfsHash)
}
