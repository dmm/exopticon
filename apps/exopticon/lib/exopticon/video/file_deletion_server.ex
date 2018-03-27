require Logger

defmodule Exopticon.Video.FileDeletionServer do
  use GenServer

  def start_link do
    GenServer.start_link(__MODULE__, [], name: __MODULE__)
  end

  def init(_) do
    schedule_work()
    {:ok, []}
  end

  def handle_info(:work, []) do
    Exopticon.Video.list_camera_groups()
    |> handle_groups()

    schedule_work()
    {:noreply, []}
  end

  defp handle_groups([]) do
  end

  defp handle_groups([group | tail]) do
    run([group.id, group.max_storage_size])

    handle_groups(tail)
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

    stat = File.stat!(file.filename)
    File.rm(file.filename)
    Exopticon.Video.delete_file(file)
    stat.size / 1024
  end

  defp schedule_work() do
    # after 5 seconds
    Process.send_after(self(), :work, 5000)
  end
end
