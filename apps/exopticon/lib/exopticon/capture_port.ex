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
    Process.flag(:trap_exit, true)
    port = Port.open({:spawn, "apps/exopticon/src/captureserver #{url} #{fps} #{video_dir} /tmp/shot.jpg"},
      [:binary, { :packet, 4 }])
#    port = Port.open({:spawn, "pwd"}, [:binary, {:packet, 4}])
    {:ok, %{port: port, id: id}}
  end

#  def handle_call(arg, _from, _names) do

 # end

  #def handle_cast(arg, _names) do

  #end

  # Handle messages from port
  def handle_info({ port, { :data, msg } }, %{port: port, id: id}) do
#    IO.puts("Got frame from: " <> Integer.to_string(id))
    %{frameJpeg: %Bson.Bin{bin: dec }, pts: pts} = Bson.decode(msg)
    ExopticonWeb.Endpoint.broadcast!("camera:"<>Integer.to_string(id), "jpg", %{ frameJpeg: dec, pts: pts})
    {:noreply, %{port: port, id: id}}
  end

  def handle_info({:EXIT, _port, reason}, state) do
    {:stop, reason, state}
  end

  def terminate(_reason, %{id: _, port: port}) do
    if Port.info(port) != nil do
      Port.close(port)
    end
    :normal
  end

end
