require Logger

defmodule Exopticon.Video.FileDeletionServer do
  use GenServer

  def start_link(arg) do
    GenServer.start_link(__MODULE__, [arg], name: __MODULE__)
  end

  def init([[_camera_group_id, _max_size] = state]) do
    schedule_work()
    {:ok, state}
  end

  def handle_info(:work, state) do
    run(state)
    schedule_work()
    {:noreply, state}
  end

  defp run([camera_group_id, max_size] = state) do
    video_size = Exopticon.Video.get_total_video_size(camera_group_id)
    max_size_kb = max_size * 1024 * 1024
    delete_amount = video_size - max_size_kb

    if delete_amount > 0 do
      files = Exopticon.Video.get_oldest_files_in_group(camera_group_id, 1000)
      delete_files(files, delete_amount)
      run(state)
    end
  end

  defp delete_files([head | tail], delete_amount) when delete_amount > 0 do
    new_amount = delete_amount - delete_file(head)
    delete_files(tail, new_amount)
  end

  defp delete_files([], delete_amount) when delete_amount > 0 do
    delete_amount
  end

  defp delete_files([_head | _tail], delete_amount) when delete_amount < 0 do
    0
  end

  def delete_file(file) do
    Logger.info(fn ->
      "Deleting file #{inspect(file)}"
    end)

    File.rm(file.filename)
    Exopticon.Video.delete_file(file)
    file.size
  end

  defp schedule_work() do
    # after 5 seconds
    Process.send_after(self(), :work, 5000)
  end
end
