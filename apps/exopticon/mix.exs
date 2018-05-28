defmodule Mix.Tasks.Compile.NativeWorkers do
  def run(_args) do
    {result, _errcode} =
      System.cmd(
        "make",
        [],
        stderr_to_stdout: true,
        cd: __DIR__ <> "/src/"
      )

    IO.binwrite(result)
  end
end

defmodule Mix.Tasks.Compile.Javascript do
  def run(_args) do
    if Mix.env() == :prod do
      {result, _errcode} =
        System.cmd("npm", ["run", "deploy"], stderr_to_stdout: true, cd: __DIR__ <> "/assets")

      IO.binwrite(result)
    end

    :ok
  end
end

defmodule Exopticon.Mixfile do
  use Mix.Project

  def project do
    [
      app: :exopticon,
      version: "0.0.1",
      elixir: "~> 1.4",
      elixirc_paths: elixirc_paths(Mix.env()),
      compilers: [:phoenix, :gettext, :native_workers] ++ Mix.compilers(),
      start_permanent: Mix.env() == :prod,
      aliases: aliases(),
      deps: deps()
    ]
  end

  # Configuration for the OTP application.
  #
  # Type `mix help compile.app` for more information.
  def application do
    [
      mod: {Exopticon.Application, []},
      extra_applications: [:logger, :runtime_tools, :comeonin, :timex],
      start_phases: [{:start, [5]}, {:admin, [4]}, {:stop, [3]}]
    ]
  end

  # Specifies which paths to compile per environment.
  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]

  # Specifies your project dependencies.
  #
  # Type `mix help deps` for examples and options.
  defp deps do
    [
      {:phoenix, "~> 1.3.0"},
      {:phoenix_pubsub, "~> 1.0"},
      {:phoenix_ecto, "~> 3.2"},
      {:postgrex, ">= 0.0.0"},
      {:phoenix_html, "~> 2.10"},
      {:phoenix_live_reload, "~> 1.0", only: :dev},
      {:gettext, "~> 0.11"},
      {:cowboy, "~> 1.0"},
      {:msgpax, "~> 2.0"},
      {:comeonin, "~> 4.0"},
      {:bcrypt_elixir, "~> 0.12.0"},
      {:credo, github: "rrrene/credo", only: [:dev, :test], runtime: false},
      {:timex, "~> 3.2.2"},
      {:exvif, in_umbrella: true}
    ]
  end

  # Aliases are shortcuts or tasks specific to the current project.
  # For example, to create, migrate and run the seeds file at once:
  #
  #     $ mix ecto.setup
  #
  # See the documentation for `Mix` for more info on aliases.
  defp aliases do
    [
      "ecto.setup": ["ecto.create", "ecto.migrate", "run priv/repo/seeds.exs"],
      "ecto.reset": ["ecto.drop", "ecto.setup"],
      test: ["ecto.create --quiet", "ecto.migrate", "test"]
    ]
  end
end
