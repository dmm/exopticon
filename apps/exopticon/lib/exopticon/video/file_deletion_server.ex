# This file is a part of Exopticon, a free video surveillance tool. Visit
# https://exopticon.org for more information.
#
# Copyright (C) 2018 David Matthew Mattli
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

require Logger

defmodule Exopticon.Video.FileDeletionServer do
  @moduledoc """
  Provides server that deletes video files as needed
  """
  use GenServer

  import Ecto.Query
  alias Exopticon.Repo

  alias Exopticon.Video

  def start_link do
    GenServer.start_link(__MODULE__, [], name: __MODULE__)
  end

  def init(_) do
    schedule_work()
    {:ok, []}
  end

  def handle_info(:work, []) do
    Logger.debug(fn -> "starting work" end)

    Video.list_camera_groups()
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
    video_size = Video.get_total_video_size(camera_group_id)
    max_size_bytes = max_size * 1024 * 1024
    delete_amount = video_size - max_size_bytes

    Logger.debug(fn ->
      "Max size bytes: #{Integer.to_string(max_size_bytes)} Delete amount: #{
        Integer.to_string(delete_amount)
      }"
    end)

    if delete_amount > 0 do
      files = Video.get_oldest_files_in_group(camera_group_id, 1000)
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

    stat = File.stat(file.filename)

    Video.delete_file(file)
    delete_video_unit(file.video_unit_id)

    if elem(stat, 0) == :ok do
      File.rm(file.filename)
      elem(stat, 1).size
    else
      0
    end
  end

  def delete_video_unit(id) do
    query =
      from(
        v in Video.VideoUnit,
        where: v.id == ^id,
        preload: [:annotations]
      )

    vu = Repo.one(query)

    Enum.each(vu.annotations, fn a ->
      Logger.info("Deleting annotation: #{Integer.to_string(a.id)}")
      Video.delete_annotation(a)
    end)

    Logger.info("Deleting video unit: " <> Integer.to_string(vu.id))
    {:ok, _} = Video.delete_video_unit(vu)
    :ok
  end

  defp schedule_work do
    # after 5 seconds
    Process.send_after(self(), :work, 5000)
  end
end
