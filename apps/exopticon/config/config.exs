# This file is responsible for configuring your application
# and its dependencies with the aid of the Mix.Config module.
#
# This configuration file is loaded before any dependency and
# is restricted to this project.
use Mix.Config

# General application configuration
config :exopticon, ecto_repos: [Exopticon.Repo]

# Configures the endpoint
config :exopticon, ExopticonWeb.Endpoint,
  url: [host: "localhost"],
  secret_key_base: "vMYaBbWb8+jRbenV5t7I5kBGyqGhA5erIbjN3Tj3u7kXrKWDVT4VCnGy0r54Q7Vo",
  render_errors: [view: ExopticonWeb.ErrorView, accepts: ~w(html json)],
  pubsub: [name: Exopticon.PubSub, adapter: Phoenix.PubSub.PG2]

# Configures Elixir's Logger
config :logger, :console,
  format: "$time $metadata[$level] $message\n",
  metadata: :all

config :mime, :types, %{
  "application/json" => ["json"]
}

# Import environment specific config. This must remain at the bottom
# of this file so it overrides the configuration defined above.
import_config "#{Mix.env()}.exs"
