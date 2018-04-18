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

    regs = Registry.lookup(Registry.CameraRegistry, cam.id)

    if regs == [] do
      Supervisor.start_child(Exopticon.CameraSupervisor, [args])
    end

    start_all_cameras(tail)
  end

  def stop_camera(id) do
    regs = Registry.lookup(Registry.CameraRegistry, id)
    pids = Enum.map(regs, fn {pid, _} -> pid end)

    Enum.map(pids, fn p ->
      Supervisor.terminate_child(Exopticon.CameraSupervisor, p)
    end)
  end
end
