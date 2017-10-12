defmodule Exopticon.PlaybackPort do
  use GenServer

  import Ecto.Query

  ### Client API
  def start_link(state, opts \\ []) do
    GenServer.start_link(__MODULE__, state, opts)
  end

  def child_spec({channel_topic, file, offset}) do
    args = {
      channel_topic,
      file.filename,
      offset
    }

    %{
      id: channel_topic,
      start: {Exopticon.PlaybackPort, :start_link, [args]},
      restart: :permanent,
      shutdown: 5000,
      type: :worker
    }
  end

  ### Server callbacks
  def init({id, filename, offset}) do
    #    offset_string = Integer.to_string(offset)
    #   IO.puts "Initializing playback port: "<> filename <> " " <> offset_string
    #  IO.puts "CWD: " <> System.cwd
    port =
      Port.open(
        {
          :spawn,
          "/home/dmm/code/1.3/exopticon/apps/exopticon/src/playbackserver #{filename} #{offset}"
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

  ### Handle messages from port
  def handle_message({%{"jpegFrame" => dec, "pts" => pts}, state}) do
    ExopticonWeb.Endpoint.broadcast!("camera:2", "jpg", %{
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
