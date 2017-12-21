defmodule Exopticon.Video.FileDeletionSupervisor do
  use Supervisor

  def start_link(arg) do
    Supervisor.start_link(__MODULE__, arg)
  end

  def init(arg) do
    children = [
      worker(Exopticon.Video.FileDeletionServer, [arg], restart: :permanent)
    ]

    supervise(children, strategy: :one_for_one)
  end
end
