using X;
using X.Y.Z;

/// A class outside the namespace.
public class OutsideClass {
    /// integer variable of this class
    public int integerVar = 5;
}

/// This namespace has a comment!
namespace A {

    public enum SomeEnum {
        A = 0,
        B,
        C,
    }

    /// Hello there
    /// This is a long complicated explanation
    /// for this api class
    public class Class {
        // wow what a brilliant string variable
        public string str;
    }

}

namespace A.B.C {
    public struct Struct {
        public float f;
    }

    private struct PrivStruct {
        private float f;
    }
}
