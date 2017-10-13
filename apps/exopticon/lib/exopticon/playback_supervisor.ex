defmodule Exopticon.PlaybackSupervisor do
  use Supervisor

  def start_link do
    Supervisor.start_link(__MODULE__, [], name: __MODULE__)
  end

  def init(_) do
    children = []
    supervise(children, strategy: :one_for_one)
  end

  def start_playback(job) do
    Supervisor.start_child(Exopticon.PlaybackSupervisor, Exopticon.PlaybackPort.child_spec(job))
  end
end
