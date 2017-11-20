defmodule Exopticon.PlaybackPort do
  use GenServer

  import Ecto.Query

  ### Client API
  def start_link(state, opts \\ []) do
    GenServer.start_link(__MODULE__, state, opts)
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

    {:ok, %{id: id, port: port, offset: offset}}
  end

  ## Handle message callback
  def handle_info({port, {:data, msg}}, %{id: id, port: port, offset: offset}) do
    {Msgpax.unpack!(msg), %{id: id, port: port, offset: offset}}
    |> handle_message
  end

  def handle_info({port, {:exit_status, status}}, %{id: id, port: port, offset: _}) do
    IO.puts("Got exit status! " <> Integer.to_string(status))
    {:stop, :normal, %{}}
  end

  ## Handle port termination
  def terminate(_reason, _state) do
    IO.puts("Terminate!")
  end

  def terminate(:normal, _state) do
    IO.puts("Normal termination of playback port!")
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
end
