defmodule Exopticon.AccountsTest do
  use ExUnit.Case
  @moduletag integration: true
  use Exopticon.DataCase

  alias Exopticon.Accounts

  describe "users" do
    alias Exopticon.Accounts.User

    @valid_attrs %{
      name: "some name",
      password: "some password",
      password_hash: "some password_hash",
      username: "some username",
      timezone: "America/Chicago"
    }
    @update_attrs %{
      name: "some updated name",
      password: "some updated password",
      password_hash: "some updated password_hash",
      username: "some updated username",
      timezone: "Africa/Abidjan"
    }
    @invalid_attrs %{name: nil, password: nil, password_hash: nil, username: nil, timezone: nil}

    def user_fixture(attrs \\ %{}) do
      {:ok, user} =
        attrs
        |> Enum.into(@valid_attrs)
        |> Accounts.create_user()

      user
    end

    test "list_users/0 returns all users" do
      user = user_fixture()
      # Clear password because it isn't used
      assert Accounts.list_users() == [%{user | password: nil}]
    end

    test "get_user!/1 returns the user with given id" do
      user = user_fixture()

      user2 = Accounts.get_user!(user.id)
      assert user.username == user2.username
      assert user.name == user2.name
      assert user.timezone == user2.timezone

    end

    test "create_user/1 with valid data creates a user" do
      assert {:ok, %User{} = user} = Accounts.create_user(@valid_attrs)
      assert user.name == "some name"
      assert user.password == "some password"
      # Don't test password hashing. Assume it's correct.
      #assert user.password_hash == "some password_hash"
      assert user.username == "some username"
    end

    test "create_user/1 with invalid data returns error changeset" do
      assert {:error, %Ecto.Changeset{}} = Accounts.create_user(@invalid_attrs)
    end

    test "update_user/2 with valid data updates the user" do
      user = user_fixture()
      assert {:ok, user} = Accounts.update_user(user, @update_attrs)
      assert %User{} = user
      assert user.name == "some updated name"
      assert user.password == "some updated password"
      # Don't test password hashing. Assume it's correct.
      #assert user.password_hash == "some updated password_hash"
      assert user.username == "some updated username"
    end

    test "update_user/2 with invalid data returns error changeset" do
      user = user_fixture()
      assert {:error, %Ecto.Changeset{}} = Accounts.update_user(user, @invalid_attrs)

      # Verify user is unchanged
      user2 = Accounts.get_user!(user.id)
      assert user.name == user2.name
      assert user.username == user2.username
      assert user.timezone == user2.timezone
    end

    test "delete_user/1 deletes the user" do
      user = user_fixture()
      assert {:ok, %User{}} = Accounts.delete_user(user)
      assert_raise Ecto.NoResultsError, fn -> Accounts.get_user!(user.id) end
    end

    test "change_user/1 returns a user changeset" do
      user = user_fixture()
      assert %Ecto.Changeset{} = Accounts.change_user(user)
    end
  end
end
