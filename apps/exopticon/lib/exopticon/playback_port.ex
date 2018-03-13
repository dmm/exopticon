defmodule Exopticon.PlaybackPort do
  use GenServer

  import Ecto.Query

  ### Client API
  def start_link({id, _, _} = state) do
    GenServer.start_link(__MODULE__, state, name: via_tuple(id))
  end

  def child_spec() do
    %{
      start: {Exopticon.PlaybackPort, :start_link, []},
      restart: :transient,
      shutdown: 5000,
      type: :worker
    }
  end

  ### Server callbacks
  def init({id, filename, offset}) do
    IO.puts("Starting playback port! " <> id <> "," <> filename)

    port =
      Port.open(
        {
          :spawn,
          "apps/exopticon/lib/exopticon/playbackworker #{filename} #{offset}"
        },
        [:binary, {:packet, 4}, :exit_status]
      )

    schedule_check()
    {:ok, %{id: id, port: port, offset: offset, timeout: System.monotonic_time()}}
  end

  ## Handle message callback
  def handle_info({port, {:data, msg}}, state) do
    {Msgpax.unpack!(msg), state}
    |> handle_message
  end

  def handle_info({port, {:exit_status, status}}, %{id: id, port: port} = state) do
    IO.puts("Got exit status! " <> Integer.to_string(status))

    ExopticonWeb.Endpoint.broadcast!(id, "stop", %{
      id: id
    })

    {:stop, :normal, %{}}
  end

  ## Handle port termination
  def terminate(_reason, _state) do
    IO.puts("Terminate!")
  end

  ### Handle messages from port
  def handle_message({%{"jpegFrame" => dec, "pts" => pts}, %{id: id, port: _, offset: _} = state}) do
    ExopticonWeb.Endpoint.broadcast!(id, "jpg", %{
      frameJpeg: Msgpax.Bin.new(dec),
      pts: pts
    })

    {:noreply, state}
  end

  def handle_message({%{"type" => "log", "message" => message}, state}) do
    IO.puts("playback message: " <> message)
    {:noreply, state}
  end

  ## Handle ack cast
  def handle_cast(:ack, state) do
    {:noreply, Map.put(state, :timeout, System.monotonic_time())}
  end

  ## Handle timeout check
  def handle_info(:timeout_check, %{timeout: timeout} = state) do
    cur = System.convert_time_unit(System.monotonic_time(), :native, :seconds)
    prev = System.convert_time_unit(timeout, :native, :seconds) + 5

    if cur > prev do
      # five seconds have passed since an Ack, close port
      {:stop, :normal, %{}}
    else
      schedule_check()
      {:noreply, state}
    end
  end

  defp schedule_check() do
    Process.send_after(self(), :timeout_check, 1000)
  end

  defp via_tuple(topic) do
    {:via, Registry, {Registry.PlayerRegistry, topic}}
  end
end
