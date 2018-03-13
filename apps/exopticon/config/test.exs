use Mix.Config

# We don't run a server during test. If one is required,
# you can enable the server option below.
config :exopticon, ExopticonWeb.Endpoint,
  http: [port: 4001],
  server: false

# Print only warnings and errors during test
config :logger, level: :warn

# Configure your database
config :exopticon, Exopticon.Repo,
  adapter: Ecto.Adapters.Postgres,
  username: System.get_env("EXOPTICON_TEST_DB_USER") || "postgres",
  password: System.get_env("EXOPTICON_TEST_DB_PASSWORD") || "postgres",
  database: "exopticon_test",
  hostname: "localhost",
  pool: Ecto.Adapters.SQL.Sandbox
