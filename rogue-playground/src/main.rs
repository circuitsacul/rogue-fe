use std::io::{Write, stdin, stdout};

use eaccess::Message;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing()?;

    let mut account = String::new();
    let mut password = String::new();

    print!("Account name: ");
    stdout().flush()?;
    stdin().read_line(&mut account)?;
    account = account.replace("\n", "");

    print!("Password: ");
    stdout().flush()?;
    stdin().read_line(&mut password)?;
    password = password.replace("\n", "");

    tracing::trace!("connecting to {:?}", eaccess::ENDPOINT);
    let tcp = TcpStream::connect(eaccess::ENDPOINT).await?;
    let mut tcp = BufReader::new(tcp);

    // ask for hash key
    tcp.write_all(eaccess::K::out().as_bytes()).await?;

    // get hash key
    let mut buf = String::new();
    tcp.read_line(&mut buf).await?;
    let res = eaccess::K::parse(buf.as_str())?;

    // login
    let hashed = eaccess::hash_password(password.bytes(), res.key.bytes());
    tcp.write_all(&eaccess::A::out(account.bytes(), hashed))
        .await?;

    // get success response
    let mut buf = String::new();
    tcp.read_line(&mut buf).await?;
    let _res = eaccess::A::parse(buf.as_str())?;

    // request instances
    tcp.write_all(eaccess::M::out().as_bytes()).await?;

    // get instances
    let mut buf = String::new();
    tcp.read_line(&mut buf).await?;
    let res = eaccess::M::parse(buf.as_str())?;

    // get info for all nodes
    let mut access = None;
    for (node, name) in &res.0 {
        if name != &"GemStone IV" {
            continue;
        }

        tcp.write_all(eaccess::G::out(node).as_bytes()).await?;

        let mut buf = String::new();
        tcp.read_line(&mut buf).await?;
        let res = eaccess::G::parse(buf.as_str())?;
        tracing::trace!("{res:?}");

        tcp.write_all(eaccess::C::out().as_bytes()).await?;

        let mut buf = String::new();
        tcp.read_line(&mut buf).await?;
        let res = eaccess::C::parse(buf.as_str())?;
        tracing::trace!("{res:?}");

        for (c_id, c_name) in res.characters {
            tracing::trace!("{c_id}, {c_name}");
            tcp.write_all(eaccess::L::out(c_id, eaccess::NProtocol::Storm).as_bytes())
                .await?;

            let mut buf = String::new();
            tcp.read_line(&mut buf).await?;
            let res = eaccess::L::parse(buf.as_str())?;
            tracing::trace!("{res:?}");
            access = Some((res.game_host.to_owned(), res.game_port, res.key.to_owned()));
            tracing::trace!("{access:?}");
        }
    }

    drop(tcp);
    let Some((host, port, key)) = access else {
        anyhow::bail!("where's my stuff");
    };
    let tcp = TcpStream::connect((host.as_str(), port as _)).await?;
    let mut tcp = BufReader::new(tcp);

    let mut line = String::new();
    tcp.read_line(&mut line).await?;
    tracing::trace!("0 -> {line}");

    tcp.write_all(format!("{key}\n/FE:WRAYTH /VERSION:1.0.1.28 /P:WIN_UNKNOWN /XML\n").as_bytes())
        .await?;
    for i in 0..3 {
        let mut line = String::new();
        tcp.read_line(&mut line).await?;
        tracing::trace!("{i} -> {line}");
    }

    tcp.write_all(b"/XML\n").await?;

    let mut buffer = String::new();

    loop {
        buffer.clear();
        tcp.read_line(&mut buffer).await?;
        print!("{buffer}")
    }

    // get node for gs4
    // let Some((gs4_node, _)) = res.0.iter().find(|(_, name)| name == &"GemStone IV") else {
    //     anyhow::bail!("GemStone IV server not found")
    // };

    // Ok(())
}

fn setup_tracing() -> anyhow::Result<()> {
    use tracing::subscriber::set_global_default;
    use tracing_subscriber::{EnvFilter, fmt::Subscriber};

    set_global_default(
        Subscriber::builder()
            .with_env_filter(EnvFilter::from_default_env())
            .finish(),
    )?;

    Ok(())
}
