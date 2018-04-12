defmodule Exopticon.PlaybackSupervisor do
  @moduledoc """
  Provides supervisor for playback port.
  """
  use Supervisor

  def start_link do
    Supervisor.start_link(__MODULE__, [], name: __MODULE__)
  end

  def init(_) do
    children = [
      worker(Exopticon.PlaybackPort, [], restart: :transient)
    ]

    supervise(children, strategy: :simple_one_for_one)
  end

  def start_playback({id, file, offset}) do
    IO.puts("Starting playback port..." <> id)
    args = {id, file.filename, offset}
    Supervisor.start_child(Exopticon.PlaybackSupervisor, [args])
  end

  def stop_playback(id) do
    regs = Registry.lookup(Registry.PlayerRegistry, id)
    pids = Enum.map(regs, fn {pid, _} -> pid end)

    Enum.map(pids, fn p ->
      Supervisor.terminate_child(Exopticon.PlaybackSupervisor, p)
    end)
  end
end
