ExUnit.configure(exclude: [integration: true])
ExUnit.start()

Ecto.Adapters.SQL.Sandbox.mode(Exopticon.Repo, :manual)

