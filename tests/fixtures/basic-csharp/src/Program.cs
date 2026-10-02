using System;

namespace Basic.CSharp
{
    public class Greeter
    {
        public string Message { get; set; }

        public string Greet(string who)
        {
            return $"hello {who}";
        }
    }
}
