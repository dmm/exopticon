defmodule Exopticon.CameraSupervisor do
  use Supervisor

  def start_link do
    Supervisor.start_link(__MODULE__, [])
  end

  def init([]) do
    children = [
      #worker(ExopticonWeb.CapturePort, [{1, "", "10", "/" }, [name: CameraPort]])
    ]

    supervise(children, strategy: :one_for_one)
  end

  def add_camera(supervisor, url, fps, dir) do

  end
end
