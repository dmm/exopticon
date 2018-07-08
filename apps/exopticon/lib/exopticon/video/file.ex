defmodule Exopticon.Video.File do
  @moduledoc """
  Provides schema for Video.File
  """
  use Ecto.Schema
  import Ecto.Changeset
  alias Exopticon.Video.File

  schema "files" do
    field(:filename, :string)
    field(:size, :integer)
    field(:video_unit_id, :id)

    timestamps()
  end

  @doc false
  def changeset(%File{} = file, attrs) do
    file
    |> cast(attrs, [
      :filename,
      :size,
      :video_unit_id
    ])
    |> validate_required([
      :filename,
      :size,
      :video_unit_id
    ])
  end
end
