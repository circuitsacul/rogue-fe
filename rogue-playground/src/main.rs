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
    for (node, name) in &res.0 {
        if name != &"GemStone IV" {
            continue;
        }

        // node info
        // tcp.write_all(eaccess::N::out(node).as_bytes()).await?;

        // let mut buf = String::new();
        // tcp.read_line(&mut buf).await?;
        // let res = eaccess::N::parse(buf.as_str())?;
        // tracing::trace!("({node}, {name}) => {res:?}");

        // request pricing model?
        // tcp.write_all(eaccess::F::out(node).as_bytes()).await?;

        // let mut buf = String::new();
        // tcp.read_line(&mut buf).await?;
        // let res = eaccess::F::parse(buf.as_str())?;
        // tracing::trace!("{res:?}");

        // what is this
        tcp.write_all(eaccess::G::out(node).as_bytes()).await?;

        let mut buf = String::new();
        tcp.read_line(&mut buf).await?;
        let res = eaccess::G::parse(buf.as_str())?;
        tracing::trace!("{res:?}");

        // and what is this lol
        // tcp.write_all(eaccess::P::out(node).as_bytes()).await?;

        // let mut buf = String::new();
        // tcp.read_line(&mut buf).await?;
        // let res = eaccess::P::parse(buf.as_str())?;
        // tracing::trace!("{node} {res:?}");

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
        }
    }

    // get node for gs4
    // let Some((gs4_node, _)) = res.0.iter().find(|(_, name)| name == &"GemStone IV") else {
    //     anyhow::bail!("GemStone IV server not found")
    // };

    Ok(())
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
