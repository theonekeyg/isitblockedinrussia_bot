use tokio_postgres::{Client, Error, NoTls};
use tokio_postgres::config::Config;
use tokio_postgres::error::SqlState;
use tokio_postgres::row::Row;

#[derive(Debug)]
pub struct BlockedDB {
    client: Client
}

impl BlockedDB {
    /// Connect to existing database with config arguments (syntax: "<arg> = <value>").
    /// Available args:
    ///                 user - The username to authenticate with. Required.
    ///             password - The password to authenticate with.
    ///               dbname - The name of the database to connect to. Defaults to the username.
    ///              options - Command line options used to configure the server.
    ///     application_name - Sets the application_name parameter on the server.
    ///              sslmode - Controls usage of TLS. If set to disable, TLS will not be used.
    ///                        If set to prefer, TLS will be used if available, but not used otherwise.
    ///                        If set to require, TLS will be forced to be used. Defaults to prefer.
    ///                 host - The host to connect to. On Unix platforms, if the host starts with a / character it is
    ///                        treated as the path to the directory containing Unix domain sockets.  Otherwise, it is
    ///                        treated as a hostname. Multiple hosts can be specified, separated by commas.  Each host
    ///                        will be tried in turn when connecting. Required if connecting with the connect method.
    ///                 port - The port to connect to. Multiple ports can be specified, separated by commas.
    ///                        The number of ports must be either 1, in which case it will be used for all hosts,
    ///                        or the same as the number of hosts. Defaults to 5432 if omitted or the empty string.
    ///      connect_timeout - The time limit in seconds applied to each socket-level connection attempt.
    ///                        Note that hostnames can resolve to multiple IP addresses,
    ///                        and this limit is applied to each address. Defaults to no timeout.
    ///           keepalives - Controls the use of TCP keepalive. A value of 0 disables keepalive and nonzero integers
    ///                        enable it. This option is ignored when connecting with Unix sockets. Defaults to on.
    ///      keepalives_idle - The number of seconds of inactivity after which a keepalive message
    ///                        is sent to the server. This option is ignored when connecting with
    ///                        Unix sockets. Defaults to 2 hours.
    /// target_session_attrs - Specifies requirements of the session. If set to read-write, the client will check that
    ///                        the transaction_read_write session parameter is set to on. This can be used to connect to
    ///                        the primary server in a database cluster as opposed to the secondary read-only mirrors.
    ///                        Defaults to all.
    ///      channel_binding - Controls usage of channel binding in the authentication process. If set to disable,
    ///                        channel binding will not be used. If set to prefer, channel binding will be used
    ///                        if available, but not used otherwise. If set to require, the authentication
    ///                        process will fail if channel binding is not used. Defaults to prefer.
    pub async fn connect(config: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // let (client, connection) = tokio_postgres::connect(config, NoTls).await?;
        let config = config.parse::<Config>()?;
        let res = config.connect(NoTls).await;
        if let Err(err) = res {
            if err.code().unwrap() == &SqlState::UNDEFINED_DATABASE {
                return Self::create(config).await;
            }
            return Err(Box::new(err));
        }
        let (client, connection) = res?;
        tokio::spawn(async move {
            if let Err(er) = connection.await {
                eprintln!("connection error: {}", er);
            }
        });
        Ok(BlockedDB { client: client })
    }

    pub async fn create(mut config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let dbname = String::from(config.get_dbname().unwrap());
        config.dbname("");
        let (client, connection) = config.connect(NoTls).await?;
        tokio::spawn(async move {
            if let Err(er) = connection.await {
                eprintln!("connection error: {}", er);
            }
        });

        client.execute(format!("CREATE DATABASE {}", dbname).as_str(), &[]).await?;
        config.dbname(dbname.as_str());
        let (client, connection) = config.connect(NoTls).await?;
        tokio::spawn(async move {
            if let Err(er) = connection.await {
                eprintln!("connection error: {}", er);
            }
        });

        // TODO: Change ip column to hold 128 bit integer instead of varchar (for IPv6 addresses)
        // or maybe ditch all IPv6 addresses at all and then use 32 bit for IPv4 only
        client.execute("CREATE TABLE blocked (
            ip VARCHAR(64) DEFAULT NULL,
            domain VARCHAR(256) DEFAULT NULL,
            url VARCHAR(256) DEFAULT NULL,
            decision_org VARCHAR(128) DEFAULT NULL,
            decision_num VARCHAR(128) DEFAULT NULL,
            decision_date VARCHAR(32) DEFAULT NULL
            )", &[]).await?;
        let db = BlockedDB { client: client };
        db.update().await?;
        Ok(db)
    }

    pub async fn get_blocked(&self, s: String) -> Result<Vec<Row>, Error> {
        let rv = self.client.query(
            "SELECT * FROM blocked WHERE ip = $1::TEXT
             OR domain = $1::TEXT",
            &[&s])
            .await?;
        Ok(rv)
    }

    /// Reset and refill blocked table with 3rd party data
    pub async fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Extremist sources: https://minjust.gov.ru/uploaded/files/exportfsm.csv
        // Blocked ips: http://raw.githubusercontent.com/zapret-info/z-i/master/dump.csv
        let blocked_uri = "http://raw.githubusercontent.com/zapret-info/z-i/master/dump-00.csv";
        let blocked = reqwest::get(blocked_uri)
            .await?
            .text()
            .await?;

        self.client.execute("TRUNCATE TABLE blocked;", &[]).await?;
        let mut lines = blocked.split('\n');
        let mut rows: Vec<Vec<String>> = Vec::with_capacity(10000);
        // Skip header
        lines.next();
        while let Some(line) = lines.next() {
            let mut cols = line.split(';');
            let mut items = Vec::with_capacity(6);
            while let Some(col) = cols.next() {
                items.push(String::from(col));
            }
            if items.len() != 6 {
                continue;
            }

            let mut ips = cols.next().unwrap().split(" | ");
            let domain = cols.next().unwrap().to_owned();
            let url = cols.next().unwrap().to_owned();
            let decision_org = cols.next().unwrap().to_owned();
            let decision_num = cols.next().unwrap().to_owned();
            let decision_date = cols.next().unwrap().to_owned();

            while let Some(ip) = ips.next() {
                if ip.is_empty() { continue; }
                rows.push(vec![
                   String::from(ip),
                   domain.clone(),
                   url.clone(),
                   decision_org.clone(),
                   decision_num.clone(),
                   decision_date.clone()
                   ]
                );
                if rows.len() >= 1000 {
                    let mut params = String::with_capacity(10000);
                    params_from_iter(&mut params, &rows);
                    let query: String = format!("INSERT INTO blocked VALUES {}", params);
                    self.client.execute(query.as_str(), &[]).await?;
                    rows.clear();
                }
            }
            if rows.len() != 0 {
                let mut params = String::with_capacity(5000);
                params_from_iter(&mut params, &rows);
                let query: String = format!("INSERT INTO blocked VALUES {}", params);
                self.client.execute(query.as_str(), &[]).await?;
                rows.clear();
            }
        }


        Ok(())
    }
}

fn params_from_iter(dest: &mut String, it: &Vec<Vec<String>>) {
    for row in it.iter() {
        *dest += &(
            "(".to_owned()
            + &row.iter().map(|x| { "'".to_owned() + x + "'" }).collect::<Vec<String>>().join(",")
            + "),"
        );
    }
    dest.pop();
}
