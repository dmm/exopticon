defmodule Exopticon.Application do
  use Application

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
      # Start your own worker by calling: Exopticon.Worker.start_link(arg1, arg2, arg3)
      # worker(Exopticon.Worker, [arg1, arg2, arg3]),
      supervisor(Exopticon.CameraSupervisor, [])
    ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: Exopticon.Supervisor]
    Supervisor.start_link(children, opts)
  end

  def start_camera(c = %Exopticon.Video.Camera{}) do
#    Supervisor.start_child(Exopticon.CameraSupervisor, [{c.id, c.rtsp_url, c.fps, 
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  def config_change(changed, _new, removed) do
    ExopticonWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
