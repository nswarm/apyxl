using Platform.Service;

namespace Platform
{

// feature: dto
public class PlatformInfo
{
    
    // feature: primitives
    public bool IsHealthy;
    public UInt64 NumUsers;
    public Service.User User;

    // feature: rpc static method
    // feature: rpc return type
    public static PlatformInfo Get()
    {
        return new PlatformInfo();
    }

    // feature: rpc
    // feature: rpc params
    // feature: namespace references
    public Service.User GetUser(User.Id id, bool isOnline) {
        return new User();
    }
}

}
