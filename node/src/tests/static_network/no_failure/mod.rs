use network::turmoil;
use network::types::SocketAddr;

#[cfg(feature = "simulation")]
mod clocks;

#[cfg(feature = "simulation")]
mod broadcast;

static LOG_INIT: std::sync::Once = std::sync::Once::new();
static TEST_MUTEX: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

/// Setup function that is only run once, even if called multiple times.
fn test_setup<'a>() -> std::sync::MutexGuard<'a, ()> {
    LOG_INIT.call_once(|| {
        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish(),
        )
        .expect("Configure tracing");
    });

    TEST_MUTEX.lock().unwrap()
}

fn node_names_to_addresses<S: AsRef<str>>(node_names: &[S]) -> Vec<SocketAddr> {
    let mut addresses = Vec::with_capacity(256);
    for node_name in node_names {
        addresses.push(SocketAddr::from((
            turmoil::lookup(node_name.as_ref()),
            9000,
        )));
    }

    addresses
}

fn wait_nodes(matrix: &mut turmoil::Sim, node_names: &[&str], timeout_secs: Option<u64>) {
    let timeout_secs = timeout_secs.unwrap_or(u64::MAX);

    let now = std::time::Instant::now();

    for name in node_names {
        while matrix.is_host_running(*name) {
            let elapsed = now.elapsed().as_secs();
            if elapsed > timeout_secs {
                tracing::info!("Testing too long... Give up.");
                return;
            }
            matrix.run().unwrap();
        }
    }
}
