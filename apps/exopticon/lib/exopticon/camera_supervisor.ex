defmodule Exopticon.CameraSupervisor do
  @moduledoc """
  Provides supervisor for capture port processes.
  """
  use Supervisor

  def start_link do
    Supervisor.start_link(__MODULE__, [], name: __MODULE__)
  end

  def init(_) do
    children = [
      worker(Exopticon.CapturePort, [], restart: :permanent)
    ]

    supervise(children, strategy: :simple_one_for_one)
  end

  def start_all_cameras([]) do
  end

  def start_all_cameras(cameras) do
    [cam | tail] = cameras
    args = {
      cam.id,
      cam.rtsp_url,
      cam.fps,
      cam.camera_group.storage_path
    }

    ret = Supervisor.start_child(Exopticon.CameraSupervisor, [args])
    start_all_cameras(tail)
  end
end
