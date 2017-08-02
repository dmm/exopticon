defmodule Exopticon.CameraSupervisor do
  use Supervisor

  import Ecto.Query

  def start_link do
    Supervisor.start_link(__MODULE__, [], name: __MODULE__)
  end

  def init(cameras) do
    children = [
      worker(Exopticon.CapturePort, [], restart: :permanent)
    ]

    supervise(children, strategy: :simple_one_for_one)
  end

  def start_all_cameras([]) do

  end

  def start_all_cameras(cameras) do
    [cam | tail] = cameras
    Supervisor.start_child(Exopticon.CameraSupervisor, [{cam.id, cam.rtsp_url, cam.fps, cam.camera_group.storage_path}])
    start_all_cameras(tail)
  end


end
