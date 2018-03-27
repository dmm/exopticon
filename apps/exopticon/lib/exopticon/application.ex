defmodule Exopticon.Application do
  use Application

  import Ecto.Query

  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  def start(_type, _args) do
    import Supervisor.Spec

    # Define workers and child supervisors to be supervised
    children = [
      # Start the Ecto repository
      supervisor(Exopticon.Repo, []),
      # Start the endpoint when the application starts
      supervisor(ExopticonWeb.Endpoint, []),
      supervisor(Registry, [:unique, Registry.PlayerRegistry]),
      supervisor(Exopticon.CameraSupervisor, []),
      supervisor(Exopticon.PlaybackSupervisor, []),
      supervisor(Exopticon.Video.FileDeletionSupervisor, [])
    ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: Exopticon.Supervisor]
    ret = Supervisor.start_link(children, opts)

    # Start Cameras
    Exopticon.Repo.all(from(camera in Exopticon.Video.Camera, preload: [:camera_group]))
    |> Exopticon.CameraSupervisor.start_all_cameras()

    ret
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  def config_change(changed, _new, removed) do
    ExopticonWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
