using System.Collections.Generic;

namespace Platform.Service
{

using SpecialId = UInt32;

[System.Serializable]
public class User
{
    public const string PubConstName = "name";
    public const string ConstName = "name";
    public static string PubStaticName = "name";
    public static string StaticName = "name";

    public Id UserId;
    public State UserState;

    public DisplayInfo Display;

    // feature: maps
    // feature: nested type dependency - generator will import social.rs
    public Dictionary<Id, Social.Friend> Friends;

    // feature: complex nested types
    public Dictionary<string, List<Inventory.Item>> EquipmentSlots;

    // feature: user type in parser config
    public SpecialId SpecialId;

    // feature: nested struct
    public struct Id
    {
        // feature: property accessor
        public UInt128 Value { get; }
    }

    // feature: nested enum
    public enum State
    {
        LoggedIn,
        Idle,
    }
}

public struct DisplayInfo
{
    public string DisplayName;
    public string Discriminator;
    public Presence Presence;
}


// feature: enum
public enum Presence
{
    Offline,
    Online,
    Invalid = 999,
}

// feature: namespace within a namespace
namespace Inventory
{
public struct Item
{
    // feature: property shorthand get
    public string Name => "name";
    
    private string _id;

    // feature: property shorthand get/set
    public string Id
    {
        get => _id;
        private set => _id = value;
    }

    // feature: property get/set {}
    private string _data;
    private string Data
    {
        get { return _data; }
        set { _data = value; }
    }
}

}
}
