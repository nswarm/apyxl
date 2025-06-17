using System.Reflection;
using Platform.Service;

// feature: chunk-local using alias.
using UserId = Platform.Service.User.Id;

// ignored.
using System.Collections;

// feature: ignore assembly info
[assembly: AssemblyVersion("1.2.3.4")]

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
    public Service.User GetUser(UserId id, bool isOnline)
    {
        return new User();
    }
}

}
