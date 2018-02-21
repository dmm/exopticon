# This file is part of Exopticon (https://github.com/dmm/exopticon).
# Copyright (c) 2018 David Matthew Mattli
#
# Exopticon is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Exopticon is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.

defmodule Exopticon.FileLibrary do
  # microseconds
  @max_gap 500 * 1000

  defp append_file([], file) do
    [
      %{
        begin_time: file.begin_time,
        end_time: file.end_time,
        type: :video
      }
    ]
  end

  defp append_file(chunks, file) do
    [last | rest] = chunks
    gap = Timex.diff(file.begin_time, last.end_time, :microseconds)

    new_chunks =
      case gap < @max_gap do
        true ->
          [
            %{
              begin_time: last.begin_time,
              end_time: file.end_time,
              type: :video
            }
          ]

        false ->
          [
            %{
              begin_time: file.begin_time,
              end_time: file.end_time,
              type: :video
            },
            %{
              begin_time: Timex.shift(last.end_time, microseconds: 1),
              end_time: Timex.shift(file.begin_time, microseconds: -1),
              type: :no_video
            },
            last
          ]
      end

    new_chunks ++ rest
  end

  defp calculate_availability(chunks, []) do
    chunks |> Enum.reverse()
  end

  defp calculate_availability(chunks, files) do
    [next_file | rest] = files
    new_chunks = append_file(chunks, next_file)

    calculate_availability(new_chunks, rest)
  end

  def calculate_availability(files) do
    calculate_availability([], files)
  end
end
