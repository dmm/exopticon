defmodule Exopticon.CapturePort do
  use GenServer

  ### Client API
  def start_link(state, opts \\ []) do
    GenServer.start_link(__MODULE__, state, opts)
  end

  def child_spec(camera) do
    args = {
      camera.id,
      camera.rtsp_url,
      camera.fps,
      camera.camera_group.storage_path
      }
    %{
      id: camera.id,
      start: {Exopticon.CapturePort, :start_link, [args]},
      restart: :permanent,
      shutdown: 5000,
      type: :worker
      }
  end


  ### Server callbacks
  def init({id, url, fps, video_dir}) do
    storage_path = video_dir
    |> Path.expand
    |> Path.join(Integer.to_string(id))
    File.mkdir_p!(storage_path)
    port = Port.open({:spawn, "apps/exopticon/src/captureserver #{url} #{fps} #{storage_path} /tmp/shot.jpg"},
      [:binary, { :packet, 4 }, :exit_status])
#    port = Port.open({:spawn, "pwd"}, [:binary, {:packet, 4}])
    {:ok, %{port: port, id: id}}
  end

#  def handle_call(arg, _from, _names) do

 # end

  #def handle_cast(arg, _names) do

  #end

  # Handle messages from port
  def handle_info({ port, { :data, msg } }, %{port: port, id: id}) do
    {Bson.decode(msg), id}
    |> handle_message

    {:noreply, %{port: port, id: id}}
  end

  def handle_message({%{frameJpeg: %Bson.Bin{bin: dec}, pts: pts}, id}) do
    ExopticonWeb.Endpoint.broadcast!("camera:"<>Integer.to_string(id), "jpg", %{ frameJpeg: dec, pts: pts})
  end

  def handle_message({%{type: "newFile", filename: filename, beginTime: beginTime}, id}) do
    IO.puts "Got new file from " <> Integer.to_string(id) <> "! " <> filename <> " at " <> beginTime
  end

  def handle_info({:EXIT, _port, reason}, state) do
    IO.puts "Capture port stoppped! The reason is: " <> Atom.to_string(reason)
    {:stop, reason, state}
  end

  def handle_info({_port, {:exit_status, status}}, state) do
    IO.puts "Exit Status: " <> Integer.to_string(status)
    {:stop, :normal, state}
  end

  def terminate(_reason, %{id: _, port: port}) do
    if Port.info(port) != nil do
      Port.close(port)
    end
    :normal
  end

end
