defmodule Exopticon.Repo.Migrations.CreateVideoUnits do
  use Ecto.Migration

  def up do
    rename table("files"), to: table("video_units")
    create table("new_files") do
      add :filename, :string, null: false
      add :size, :integer
      add :video_unit_id, references(:video_units, on_delete: :nothing)

      timestamps()
    end

    flush()

    # convert data
    execute """
    INSERT INTO new_files (filename, size, video_unit_id)
    SELECT filename, size, id AS video_unit_id
    FROM video_units;
    """

    rename table("new_files"), to: table("files")

    # remove old columns
    alter table(:video_units) do
      remove :filename
      remove :size
    end
  end

  def down do

  end
end
